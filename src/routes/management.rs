use std::sync::Mutex;

use askama::Template;
use fqdn::fqdn;
use rand::rngs::StdRng;
use rocket::{data::ToByteUnit, fairing::AdHoc, get, post, routes, serde::json::Json, Data, State};
use sea_orm::{ActiveModelTrait, DbConn};

use crate::{
    account::{
        self, AdminAccount, AdminAccountActiveModel, PasswordAccount, PublicKey, SigningAccount,
    },
    client_response::ClientResponse,
    route53::Route53,
    utils::{generate_random_password, Credentials},
};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Mount management routes", |rocket| {
        Box::pin(async move {
            rocket.mount(
                "/mgmt",
                routes![add_signing_domain, add_password_domain, new_admin],
            )
        })
    })
}

#[get("/add-domain/password/<domain>")]
async fn add_password_domain(
    domain: String,
    admin: AdminAccount,
    route53: &State<Route53>,
    db: &State<DbConn>,
    rng: &State<Mutex<StdRng>>,
) -> ClientResponse {
    let domain = domain.trim();
    if !route53.inner().domain_included(&fqdn!(&domain)) {
        ::log::warn!(
            "The admin \"{}\" attempted to add the following domain, which is not supported: {}",
            admin.get_user(),
            domain
        );
        return ClientResponse::NotAcceptable(String::from(
            "The domain name provided is not available for dynamic DNS.",
        ));
    }

    match account::exists(domain, db.inner()).await {
        Ok(exists) => {
            if exists {
                ::log::warn!(
                "The admin \"{}\" attempted to add the domain \"{}\", but it already is in use.",
                admin.get_user(),
                domain
            );
                return ClientResponse::Conflict(String::from(
                    "The domain name provided is already in use.",
                ));
            }
        }
        Err(e) => {
            ::log::error!("An error occured communicating with the database: {}", e);
            return ClientResponse::InternalServerError(String::new());
        }
    }

    let (password, password_hash) = generate_random_password(rng.inner());

    let _ = PasswordAccount::create_account(domain, &password_hash, &admin, db.inner()).await;

    ::log::warn!(
        "The admin \"{}\" added this new domain: {}",
        admin.get_user(),
        domain
    );
    ClientResponse::Ok(
        CommandDownload::new(domain.to_string(), password.clone())
            .render()
            .unwrap(),
    )
}

#[post("/add-domain/signing/<domain>", data = "<signature>")]
async fn add_signing_domain(
    domain: String,
    signature: Data<'_>,
    admin: AdminAccount,
    route53: &State<Route53>,
    db: &State<DbConn>,
) -> ClientResponse {
    let domain = domain.trim();
    if !route53.inner().domain_included(&fqdn!(&domain)) {
        ::log::warn!(
            "The admin \"{}\" attempted to add the following domain, which is not supported: {}",
            admin.get_user(),
            domain
        );
        return ClientResponse::NotAcceptable(String::from(
            "The domain name provided is not available for dynamic DNS.",
        ));
    }

    match account::exists(domain, db.inner()).await {
        Ok(exists) => {
            if exists {
                ::log::warn!(
                "The admin \"{}\" attempted to add the domain \"{}\", but it already is in use.",
                admin.get_user(),
                domain
            );
                return ClientResponse::Conflict(String::from(
                    "The domain name provided is already in use.",
                ));
            }
        }
        Err(e) => {
            ::log::error!("An error occured communicating with the database: {}", e);
            return ClientResponse::InternalServerError(String::new());
        }
    }

    let bytes = match signature.open(10.kilobytes()).into_bytes().await {
        Err(e) => {
            log::error!("Error streaming bytes from included file in request: {}", e);
            return ClientResponse::InternalServerError("".to_string());
        }
        Ok(v) => {
            if v.is_complete() {
                v.into_inner()
            } else {
                log::warn!(
                    "The admin \"{}\" attempted to upload a key with over 10KB in size",
                    admin.get_user()
                );
                return ClientResponse::NotAcceptable(
                    "The public key uploaded can't be over 10KB in size.".to_string(),
                );
            }
        }
    };

    if bytes.is_empty() {
        log::info!(
            "The admin \"{}\" attempted to create a signing account without a public key.",
            admin.get_user()
        );
        return ClientResponse::NotAcceptable(
            "The public key uploaded can't be empty.".to_string(),
        );
    }

    let public_key = PublicKey::public_key_from_pem(&bytes).unwrap();

    let _ = SigningAccount::create_account(domain, &public_key, &admin, db.inner()).await;

    ::log::warn!(
        "The admin \"{}\" added the domain \"{}\" with a public key",
        admin.get_user(),
        domain,
    );
    ClientResponse::Ok(String::default())
}

#[post("/admin/new", data = "<credentials>")]
async fn new_admin(
    credentials: Json<Credentials>,
    admin: AdminAccount,
    db: &State<DbConn>,
) -> ClientResponse {
    if admin.get_user() != "admin" {
        ::log::warn!(
            "The user {} attempted to create an admin user, but is not allowed.",
            admin.get_user()
        );
        return ClientResponse::Unauthorized(
            "Your user is not allowed to execute this operation.".to_string(),
        );
    }

    let account: AdminAccountActiveModel = credentials.into_inner().into();
    let result = account.insert(db.inner()).await;
    if let Err(e) = result {
        match e {
            sea_orm::error::DbErr::RecordNotInserted => {
                ::log::warn!("The admin attempted to create a new user, but it already exists.");
                return ClientResponse::NotAcceptable(
                    "This username already has an account.".to_string(),
                );
            }
            _ => {
                ::log::warn!("Cannot create new admin: {}", e.to_string());
                return ClientResponse::InternalServerError(
                    "An error prevented the requested from being processed.".to_string(),
                );
            }
        }
    }

    ClientResponse::Ok(format!("Account {} created.", result.unwrap().user))
}

#[derive(Template)]
#[template(path = "command_download.txt")]
pub struct CommandDownload {
    username: String,
    password: String,
}

impl CommandDownload {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    fn url() -> String {
        let mut url: String = std::env::var("FT_DDNS_BASE_URL").unwrap_or_default();
        url.push_str("/ft-ddns.sh");

        url
    }
}

impl From<Credentials> for CommandDownload {
    fn from(value: Credentials) -> Self {
        Self::new(value.username, value.password)
    }
}
