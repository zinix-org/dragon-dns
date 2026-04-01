/*
 * Copyright 2026-present Viktor Popp
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::token::verify_token;
use chrono::{DateTime, Local, TimeDelta};
use cloudflare::{
    endpoints::{
        dns::dns::{
            DnsContent, DnsRecord, ListDnsRecords, ListDnsRecordsOrder, ListDnsRecordsParams,
            UpdateDnsRecord, UpdateDnsRecordParams,
        },
        zones::zone::{ListZones, ListZonesOrder, ListZonesParams, Zone},
    },
    framework::{
        Environment, OrderDirection,
        auth::Credentials,
        client::{ClientConfig, blocking_api::HttpApiClient},
    },
};
use croner::Cron;
use std::{
    env,
    error::Error,
    net::Ipv4Addr,
    panic::{self, PanicHookInfo},
    path::Path,
    str::FromStr,
    thread::{self, sleep},
    time::Duration,
};

mod logger;
mod token;

fn get_ip4() -> Ipv4Addr {
    let ip4_raw = reqwest::blocking::get("https://checkip.amazonaws.com/")
        .expect("failed to fetch IP address response")
        .text()
        .expect("failed to fetch IP address response");
    ip4_raw
        .as_str()
        .trim()
        .parse()
        .expect("failed to pass IP address")
}

fn list_zones(api_client: &HttpApiClient) -> Vec<Zone> {
    let endpoint = ListZones {
        params: ListZonesParams {
            name: None,
            status: None,
            page: None,
            per_page: Some(50),
            order: Some(ListZonesOrder::Name),
            direction: Some(OrderDirection::Ascending),
            search_match: None,
        },
    };

    let response = api_client.request(&endpoint);

    match response {
        Ok(success) => success.result,
        Err(error) => panic!("failed to list zones: {:#?}", error),
    }
}

fn list_records(api_client: &HttpApiClient, id: &str) -> Vec<DnsRecord> {
    let endpoint = ListDnsRecords {
        zone_identifier: id,
        params: ListDnsRecordsParams {
            record_type: None,
            name: None,
            page: None,
            per_page: Some(500), // I really hope people don't have more than that
            order: Some(ListDnsRecordsOrder::Name),
            direction: Some(OrderDirection::Ascending),
            search_match: None,
        },
    };

    let response = api_client.request(&endpoint);

    match response {
        Ok(success) => success.result,
        Err(error) => panic!("failed to list DNS records from {}: {:#?}", id, error),
    }
}

struct App {
    ip4_domains: Vec<String>,
    cache_expiration: Duration,
    cached_last_time: DateTime<Local>,
    cached_ip4: Ipv4Addr,
    update_cron: Cron,
    machine_id: String,
    api_client: HttpApiClient,
}

impl App {
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let now = Local::now();
            let next = self.update_cron.find_next_occurrence(&now, false)?;

            let ip4 = get_ip4();

            if ip4 != self.cached_ip4
                || now - self.cached_last_time > TimeDelta::from_std(self.cache_expiration)?
            {
                log::info!("updating records. IPv4: {}, time: {}", ip4, now);
                self.cached_ip4 = ip4;
                self.cached_last_time = now;
                self.update_records()?;
            }
            sleep((next - now).to_std()?);
        }
    }

    fn update_records(&mut self) -> Result<(), Box<dyn Error>> {
        let zones = list_zones(&self.api_client);
        for zone in zones {
            let records = list_records(&self.api_client, &zone.id);
            for record in records {
                if self.ip4_domains.contains(&record.name)
                    || (self.machine_id != String::new()
                        && record
                            .comment
                            .clone()
                            .unwrap()
                            .contains(format!("DDNS_ID={}", &self.machine_id).as_str()))
                {
                    self.update_ip4_record(&zone, &record);
                }
            }
        }
        Ok(())
    }

    fn update_ip4_record(&mut self, zone: &Zone, record: &DnsRecord) {
        let endpoint = UpdateDnsRecord {
            zone_identifier: &zone.id,
            identifier: &record.id,
            params: UpdateDnsRecordParams {
                ttl: Some(record.ttl),
                proxied: Some(record.proxied),
                name: &record.name,
                content: DnsContent::A {
                    content: self.cached_ip4,
                },
                comment: record.comment.as_deref(),
                tags: Some(&record.tags),
            },
        };

        let response = self.api_client.request(&endpoint);

        match response {
            Ok(_) => {}
            Err(_) => panic!("failed to update DNS record"),
        }
    }
}

fn panic_hook(info: &PanicHookInfo) {
    log::error!("{}", info.payload_as_str().unwrap());
    loop {
        thread::park();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if Path::new(".env").exists() {
        dotenvy::dotenv()?;
    }
    logger::init()?;
    panic::set_hook(Box::new(panic_hook));

    let mut token = String::new();
    let mut domains: Vec<String> = vec![];
    let mut ip4_domains: Vec<String> = vec![];
    let mut cache_expiration = Duration::from_hours(6);
    let mut update_cron: Cron = Cron::from_str("* * * * *")?;
    let mut machine_id = String::new();

    for (key, val) in env::vars() {
        match key.as_str() {
            "CLOUDFLARE_API_TOKEN" => token = val.clone(),
            "DOMAINS" => {
                domains = val.split(',').map(|s| s.trim().to_string()).collect();
            }
            "IP4_DOMAINS" => {
                ip4_domains = val.split(',').map(|s| s.trim().to_string()).collect();
            }
            "CACHE_EXPIRATION" => {
                cache_expiration = *val
                    .parse::<humantime::Duration>()
                    .expect("failed to parse CACHE_EXPIRATION")
            }
            "UPDATE_CRON" => update_cron = Cron::from_str(&key)?,
            "MACHINE_ID" => machine_id = val.clone(),
            "LOG_LEVEL" => logger::set_level(match val.to_lowercase().as_str() {
                "error" => log::Level::Error,
                "warn" => log::Level::Warn,
                "info" => log::Level::Info,
                "debug" => log::Level::Debug,
                "trace" => log::Level::Trace,
                _ => log::Level::Debug,
            })?,
            _ => {}
        }
    }

    if token.is_empty() {
        panic!("could not find CLOUDFLARE_API_TOKEN variable in environment");
    }

    verify_token(&token)?;

    let credentials = Credentials::UserAuthToken {
        token: token.clone(),
    };

    ip4_domains.extend(domains);

    let mut app = App {
        ip4_domains,
        cache_expiration,
        cached_last_time: DateTime::default(),
        cached_ip4: Ipv4Addr::new(0, 0, 0, 0),
        update_cron,
        machine_id,
        api_client: HttpApiClient::new(
            credentials,
            ClientConfig::default(),
            Environment::Production,
        )?,
    };

    app.run()
}
