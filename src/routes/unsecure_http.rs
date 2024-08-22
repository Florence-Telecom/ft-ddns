use fqdn::fqdn;
use rocket::{fairing::AdHoc, get, routes, State};

use crate::{
    account::{Account, SigningAccount},
    client_response::ClientResponse,
    ip::IP,
    route53::Route53,
};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Mount unsecure routes", |rocket| {
        Box::pin(async move { rocket.mount("/unsecure", routes![set_record]) })
    })
}

#[get("/nic/update")]
async fn set_record(a: SigningAccount, ip: IP, route53: &State<Route53>) -> ClientResponse {
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
