mod account;
mod client_response;
mod init;
mod ip;
mod route53;
mod routes;
mod utils;

use std::net::{IpAddr, Ipv4Addr};

use rocket::{config::LogLevel, launch, Build, Rocket};
use utils::stage_rng;

#[launch]
async fn rocket() -> Rocket<Build> {
    init::log::setup_logger();

    let figment = rocket::Config::figment()
        .merge(("address", IpAddr::V4(Ipv4Addr::UNSPECIFIED)))
        .merge(("log_level", LogLevel::Critical));

    rocket::custom(figment)
        .attach(init::db::stage())
        .attach(route53::stage())
        .attach(stage_rng())
        .attach(routes::secure_http::stage())
        .attach(routes::unsecure_http::stage())
        .attach(routes::management::stage())
}
