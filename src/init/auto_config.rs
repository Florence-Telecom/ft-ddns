#![cfg(feature = "aws_auto_config")]
use std::net::Ipv4Addr;
use std::str::FromStr;

use fqdn::FQDN;
use reqwest::header::HeaderMap;
use rocket::fairing::AdHoc;

use crate::route53::Route53;

pub fn autoset_dns() -> AdHoc {
    AdHoc::on_ignite("DNS auto-configuration", |rocket| {
        Box::pin(async move {
            let route53 = rocket
                .state::<Route53>()
                .expect("Must initialize Route53 before configuring DNS.");

            let web_client = reqwest::Client::new();
            let imds_token = get_imds_token(&web_client).await;

            if let Some(private_domain) = get_private_domain() {
                if !route53.domain_included(&private_domain) {
                    panic!("The domain name \"{private_domain}\" isn't available in the current configuration.")
                }
                let private_ip = get_private_ip(&web_client, &imds_token).await;

                if !route53
                    .upsert_a_resource_record(private_domain.clone(), private_ip)
                    .await
                    .is_ok()
                {
                    panic!("Failed to set {private_domain} to private IP.");
                }

                log::warn!("Auto-configured {private_domain} to {private_ip}");
            }

            if let Some(public_domain) = get_public_domain() {
                if !route53.domain_included(&public_domain) {
                    panic!("The domain name \"{public_domain}\" isn't available in the current configuration.")
                }
                if let Some(public_ip) = get_public_ip(&web_client, &imds_token).await {
                    if !route53
                        .upsert_a_resource_record(public_domain.clone(), public_ip)
                        .await
                        .is_ok()
                    {
                        panic!("Failed to set {public_domain} to public IP.");
                    }

                    log::warn!("Auto-configured {public_domain} to {public_ip}");
                } else {
                    panic!(
                        "No public IP available for the virtual machine. Couldn't set public IP domain. Check if system has a public IPv4 address on AWS."
                    );
                }
            }

            rocket
        })
    })
}

async fn get_public_ip(client: &reqwest::Client, imds_token: &str) -> Option<Ipv4Addr> {
    log::info!("Fetching public IP");

    let mut headers = HeaderMap::with_capacity(1);
    headers.insert("X-aws-ec2-metadata-token", imds_token.parse().unwrap());

    client
        .get("http://169.254.169.254/latest/meta-data/public-ipv4")
        .headers(headers)
        .send()
        .await
        .expect("Unable to get IMDSv2 API token.")
        .text()
        .await
        .ok()
        .map(|ip| Ipv4Addr::from_str(&ip).unwrap())
}

async fn get_private_ip(client: &reqwest::Client, imds_token: &str) -> Ipv4Addr {
    log::info!("Fetching private IP");

    let mut headers = HeaderMap::with_capacity(1);
    headers.insert("X-aws-ec2-metadata-token", imds_token.parse().unwrap());

    client
        .get("http://169.254.169.254/latest/meta-data/local-ipv4")
        .headers(headers)
        .send()
        .await
        .expect("Unable to get IMDSv2 API token.")
        .text()
        .await
        .ok()
        .map(|ip| Ipv4Addr::from_str(&ip).unwrap())
        .expect("Failed to retrieve private IPv4 address from IMDS.")
}

async fn get_imds_token(client: &reqwest::Client) -> String {
    log::debug!("Fetching IMDSv2 token");

    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        "X-aws-ec2-metadata-token-ttl-seconds",
        "30".parse().unwrap(),
    );

    client
        .put("http://169.254.169.254/latest/api/token")
        .headers(headers)
        .send()
        .await
        .expect("Unable to get IMDSv2 API token.")
        .text()
        .await
        .unwrap()
}

fn get_public_domain() -> Option<FQDN> {
    std::env::var("FT_DDNS_PUBLIC_DOMAIN")
        .map(string_to_fqdn)
        .ok()
}

fn get_private_domain() -> Option<FQDN> {
    std::env::var("FT_DDNS_PRIVATE_DOMAIN")
        .map(string_to_fqdn)
        .ok()
}

fn string_to_fqdn(domain: String) -> FQDN {
    fqdn::fqdn!(&domain)
}
