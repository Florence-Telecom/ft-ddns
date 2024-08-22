/// The password account database entity
///
/// Password accounts are simple to install, but require the connection to be encrypted
use rocket::{
    http,
    request::{self, FromRequest},
    Request,
};
use rocket_basicauth::BasicAuth;
use sea_orm::{entity::prelude::*, Set};

use crate::{account::AdminAccount, utils::compare_with_hash};

use super::Account;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "password_account")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// The domain name that the account can overwrite
    /// Also serves as the username for the account
    pub domain: String,
    /// The password hash of the account to authenticate requests
    pub password_hash: String,
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

pub struct PasswordAccount(String);

impl Account for PasswordAccount {
    fn get_domain(&self) -> &str {
        &self.0
    }
}

impl PasswordAccount {
    pub fn get_domain(&self) -> &str {
        &self.0
    }

    pub async fn exists(domain: &str, db: &DbConn) -> Result<bool, DbErr> {
        Entity::find()
            .filter(Column::Domain.eq(domain))
            .one(db)
            .await
            .map(|v| v.is_some())
    }

    pub async fn create_account(
        domain: &str,
        password_hash: &str,
        created_by: &AdminAccount,
        db: &DbConn,
    ) -> Result<(), DbErr> {
        let account = ActiveModel {
            disabled: Set(Some(false)),
            domain: Set(domain.to_owned()),
            created_by: Set(created_by.get_user().to_string()),
            password_hash: Set(password_hash.to_owned()),
        };

        account.insert(db).await?;

        Ok(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PasswordAccount {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        let db: &DbConn = request.rocket().state::<DbConn>().unwrap();
        let auth: BasicAuth;
        match request.guard::<BasicAuth>().await {
            request::Outcome::Error(_) => {
                return request::Outcome::Error((http::Status::BadRequest, ()))
            }
            request::Outcome::Forward(_) => {
                return request::Outcome::Error((http::Status::BadRequest, ()))
            }
            request::Outcome::Success(a) => auth = a,
        }

        let account: Option<Model> = Entity::find()
            .filter(Column::Domain.eq(&auth.username))
            .one(db)
            .await
            .unwrap();

        match account {
            None => request::Outcome::Error((http::Status::Unauthorized, ())),
            Some(d) => match compare_with_hash(&auth.password, &d.password_hash) {
                Err(_) => request::Outcome::Error((http::Status::Unauthorized, ())),
                Ok(()) => request::Outcome::Success(PasswordAccount(d.domain)),
            },
        }
    }
}
