use std::{env, net::Ipv4Addr, str::FromStr};

use crate::client_response::ClientResponse;
use aws_config::Region;
use aws_sdk_route53 as r53;
use fqdn::FQDN;
use fqdn_trie::FqdnTrieMap;
use r53::{
    error::ProvideErrorMetadata,
    types::{Change, ChangeBatch, ResourceRecord, ResourceRecordSet},
};
use rocket::fairing::AdHoc;

macro_rules! unwrap_or_return {
    ( $e:expr, $alt:expr ) => {
        match $e {
            Ok(x) => x,
            Err(_) => return $alt,
        }
    };
}

pub struct Route53 {
    client: r53::Client,
    hosted_zone_map: FqdnTrieMap<FQDN, Option<String>>,
}

impl Route53 {
    #[inline]
    pub fn domain_included(&self, domain: &FQDN) -> bool {
        self.hosted_zone_map.lookup(domain).is_some()
    }

    pub async fn upsert_a_resource_record(&self, domain: FQDN, ip: Ipv4Addr) -> ClientResponse {
        let rr = ResourceRecordSet::builder()
            .name(domain.to_string())
            .r#type(r53::types::RrType::A)
            .ttl(180)
            .resource_records(
                ResourceRecord::builder()
                    .value(ip.to_string())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let change: r53::types::Change = Change::builder()
            .action(r53::types::ChangeAction::Upsert)
            .resource_record_set(rr)
            .build()
            .unwrap();

        self.send_request(change, &domain, &ip).await
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    async fn send_request(
        &self,
        change: r53::types::Change,
        domain: &FQDN,
        ip: &Ipv4Addr,
    ) -> ClientResponse {
        #[cfg(feature = "read_only_aws")]
        {
            log::error!(
                "Would have submitted the following change to AWS Route 53: {:?}",
                change
            );

            return ClientResponse::Ok(String::from("Would have record updated on AWS Route 53."));
        }
        unwrap_or_return!(
        self.client
        .change_resource_record_sets()
        .hosted_zone_id(
            unwrap_or_return!(
                self.hosted_zone_map
                    .lookup(domain)
                    .clone()
                    .ok_or(()),
                ClientResponse::NotAcceptable(String::from("The domain requested is not in any hosted zone that is enabled for dynamic DNS."))
            )
        )
        .change_batch(
            ChangeBatch::builder()
                .changes(change)
                .build()
                .unwrap()
        )
        .send()
        .await
        .inspect_err(|e| { log::warn!("AWS change error: {}", e.message().unwrap_or("No error detail provided.")); }),
        ClientResponse::ServiceUnavailable(String::from("Failed to submit domain change to Route53."))
    );
        log::info!("Updated {} to IP {}", domain, ip);
        ClientResponse::Ok(String::from("Record updated on AWS Route 53."))
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Setting up AWS Route 53 connection", |rocket| {
        Box::pin(async move {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(Region::new("ca-central-1"))
                .load()
                .await;

            let client = r53::Client::new(&config);

            let mut hosted_zone_search = client.list_hosted_zones();

            if env::var("USE_PRIVATE_HOSTED_ZONE").is_ok_and(|v| !v.is_empty()) {
                hosted_zone_search = hosted_zone_search
                    .hosted_zone_type(r53::types::HostedZoneType::PrivateHostedZone)
            }

            let result = hosted_zone_search
                .send()
                .await
                .expect("Couldn't fetch the list of zones from AWS");

            let mut hosted_zone_ids: Vec<String> = env::var("HOSTED_ZONE_ID_LIST")
                .expect("HOSTED_ZONE_ID_LIST environment variable is not set.")
                .split(';')
                .map(|s| s.to_string())
                .collect();

            log::debug!(
                "Received the following hosted zone IDs for dynamic dns service: {:?}",
                hosted_zone_ids
            );

            let mut hosted_zone_map: FqdnTrieMap<FQDN, Option<String>> =
                FqdnTrieMap::with_capacity(None, hosted_zone_ids.len());

            log::debug!(
                "Received {} hosted zone from AWS",
                result.hosted_zones.len()
            );

            for hz in result.hosted_zones {
                log::debug!("AWS Hosted Zone name: \"{}\" with ID: {}", &hz.name, &hz.id);
                if let Some(index) = hosted_zone_ids.iter().position(|i| hz.id.contains(i)) {
                    log::info!(
                        "Domain zone \"{}\" available for services with hosted zone {}",
                        &hz.name,
                        &hz.id
                    );
                    hosted_zone_ids.remove(index);
                    hosted_zone_map.insert(FQDN::from_str(&hz.name).unwrap(), Some(hz.id));
                }
            }

            if !hosted_zone_ids.is_empty() {
                log::error!("Couldn't match the following hosted zones: {hosted_zone_ids:?}");
            }

            hosted_zone_map.shrink_to_fit();

            rocket.manage(Route53 {
                client,
                hosted_zone_map,
            })
        })
    })
}
