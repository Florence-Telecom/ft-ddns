use std::net::IpAddr;

/// The signature account database entity
///
/// Signature accounts are used when a device cannot use secure connections.
/// Requires setting up with a keypair where the public key is stored in the database.
use base64::prelude::*;
use chrono::Utc;
use openssl::hash::MessageDigest;
use openssl::pkey::PKeyRef;
use openssl::pkey::Public;
use openssl::sign::Verifier;
use rocket::{
    http,
    request::{self, FromRequest},
    Request,
};
use sea_orm::{entity::prelude::*, Set};

use crate::account::AdminAccount;

use super::Account;

/// Type public key
pub type PublicKey = openssl::pkey::PKey<Public>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "signing_account")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// The domain name that the account can overwrite
    /// Also serves as the username for the account
    pub domain: String,
    /// The public key of the account
    /// Used to verify the signature of the requests
    pub public_key: String,
    /// The admin that created the account
    pub created_by: String,
    /// If the account is disabled
    ///
    /// Must be true to disable the account
    /// Null or false means the account is enabled
    pub disabled: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct SigningAccount(String);

impl Account for SigningAccount {
    fn get_domain(&self) -> &str {
        &self.0
    }
}

impl SigningAccount {
    pub async fn exists(domain: &str, db: &DbConn) -> Result<bool, DbErr> {
        Entity::find()
            .filter(Column::Domain.eq(domain))
            .one(db)
            .await
            .map(|v| v.is_some())
    }

    pub async fn create_account(
        domain: &str,
        pub_key: &PKeyRef<Public>,
        created_by: &AdminAccount,
        db: &DbConn,
    ) -> Result<(), DbErr> {
        let signature_account = ActiveModel {
            disabled: Set(Some(false)),
            domain: Set(domain.to_owned()),
            created_by: Set(created_by.get_user().to_string()),
            public_key: Set(String::from_utf8(pub_key.public_key_to_pem().unwrap()).unwrap()),
        };

        signature_account.insert(db).await?;

        Ok(())
    }

    pub async fn find_key_by_domain(domain: &str, db: &DbConn) -> Result<Option<PublicKey>, DbErr> {
        Entity::find()
            .filter(Column::Domain.eq(domain))
            .one(db)
            .await
            .map(|r| r.map(|m| PublicKey::public_key_from_pem(m.public_key.as_bytes()).unwrap()))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SigningAccount {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        let db: &DbConn = request.rocket().state::<DbConn>().unwrap();
        let ip: IpAddr = if let Some(ip) = request.client_ip() {
            ip
        } else {
            log::warn!("Request had no IP.");
            return request::Outcome::Error((http::Status::BadRequest, ()));
        };

        let headers = request.headers();
        let date = headers.get_one("Ftddns-Date");
        let domain = headers.get_one("Ftddns-Domain").map(String::from);
        let signature = headers.get_one("Ftddns-Signature");

        let (date_str, domain, signature): (&str, String, &str) =
            if date.is_none() || domain.is_none() || signature.is_none() {
                log::warn!("{ip}: Missing headers");
                return request::Outcome::Error((http::Status::PreconditionFailed, ()));
            } else {
                (date.unwrap(), domain.unwrap(), signature.unwrap())
            };

        if let Ok(dt) =
            chrono::DateTime::parse_from_rfc3339(date_str).map(|d| d.with_timezone(&Utc))
        {
            const SIGNATURE_TIME_MARGIN: i64 = 60;
            if dt > Utc::now() + chrono::Duration::seconds(SIGNATURE_TIME_MARGIN) {
                log::warn!("{ip}'s signature date is in the future for {domain}");
                return request::Outcome::Error((http::Status::NotAcceptable, ()));
            }

            if dt < Utc::now() - chrono::Duration::seconds(SIGNATURE_TIME_MARGIN) {
                log::warn!("{ip}'s signature date is in the past for {domain}");
                return request::Outcome::Error((http::Status::NotAcceptable, ()));
            }
        } else {
            log::warn!("Invalid date format from {ip} for {domain}");
            return request::Outcome::Error((http::Status::BadRequest, ()));
        }

        let public_key: PublicKey = if let Ok(result) = Self::find_key_by_domain(&domain, db).await
        {
            if let Some(public_key) = result {
                public_key
            } else {
                log::warn!("Domain requested by {ip} does not exist in the system: {domain}");
                return request::Outcome::Error((http::Status::NotFound, ()));
            }
        } else {
            log::error!("Database error while serving {ip}");
            return request::Outcome::Error((http::Status::InternalServerError, ()));
        };

        let binary_signature = if let Ok(binary) = BASE64_STANDARD.decode(signature.as_bytes()) {
            binary
        } else {
            log::warn!("Invalid base64 encoding sent by {ip} while requesting {domain}");
            return request::Outcome::Error((http::Status::BadRequest, ()));
        };

        let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key).unwrap();

        let verification =
            verifier.verify_oneshot(&binary_signature, format!("{date_str};{domain}").as_bytes());

        match verification {
            Err(_) => {
                log::warn!("Signature verification error for {domain} from {ip}");
                return request::Outcome::Error((http::Status::Unauthorized, ()));
            }
            Ok(v) => {
                if !v {
                    log::warn!("Signature verification failed for {domain} from {ip}");
                    return request::Outcome::Error((http::Status::Unauthorized, ()));
                }
            }
        };

        request::Outcome::Success(Self(domain))
    }
}
