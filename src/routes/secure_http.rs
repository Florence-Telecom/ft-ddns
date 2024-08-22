use askama::Template;
use fqdn::fqdn;
use rocket::{fairing::AdHoc, get, post, routes, serde::json::Json, State};

use crate::{
    account::PasswordAccount, client_response::ClientResponse, ip::IP, route53::Route53,
    utils::Credentials,
};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Mount secure routes", |rocket| {
        Box::pin(async move {
            rocket.mount(
                "/secure",
                routes![set_record, shell_program, shell_program_empty],
            )
        })
    })
}

#[get("/nic/update")]
async fn set_record(a: PasswordAccount, ip: IP, route53: &State<Route53>) -> ClientResponse {
    log::info!(
        "Attempting to update DNS {} to {}",
        a.get_domain(),
        ip.get()
    );
    route53
        .inner()
        .upsert_a_resource_record(fqdn!(a.get_domain()), ip.get())
        .await
}

#[get("/ft-ddns.sh")]
fn shell_program_empty() -> FtDdnsProgram {
    FtDdnsProgram::empty()
}

#[post("/ft-ddns.sh", data = "<credentials>")]
fn shell_program(credentials: Json<Credentials>) -> FtDdnsProgram {
    credentials.into_inner().into()
}

#[derive(Template)]
#[template(path = "ft-ddns.txt")]
pub struct FtDdnsProgram {
    username: String,
    password: String,
}

impl From<Credentials> for FtDdnsProgram {
    fn from(value: Credentials) -> Self {
        Self {
            username: value.username,
            password: value.password,
        }
    }
}

impl FtDdnsProgram {
    pub fn empty() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
        }
    }
}

impl FtDdnsProgram {
    fn url() -> String {
        let mut url: String = std::env::var("FT_DDNS_BASE_URL").unwrap_or_default();
        url.push_str("/nic/update");

        url
    }
}
