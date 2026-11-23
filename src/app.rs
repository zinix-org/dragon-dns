use crate::{
    config::RuntimeConfig,
    error::AppError,
    utils::{format_duration, get_ip4},
};
use chrono::{DateTime, Local};
use cloudflare::{
    endpoints::{
        account::user::GetUserTokenStatus,
        dns::dns::{
            DnsContent, ListDnsRecords, ListDnsRecordsOrder, ListDnsRecordsParams, UpdateDnsRecord,
            UpdateDnsRecordParams,
        },
        zones::zone::{ListZones, ListZonesOrder, ListZonesParams},
    },
    framework::{
        Environment, OrderDirection,
        auth::Credentials,
        client::{ClientConfig as CloudflareConfig, async_api::Client as CloudflareClient},
    },
};
use color_eyre::{Result, eyre::bail};
use std::{net::Ipv4Addr, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error, info, trace};

pub struct App {
    api_client: CloudflareClient,
    config: RuntimeConfig,

    cached_ip4: Ipv4Addr,
    updated_last_time: DateTime<Local>,
}

impl App {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let credentials = Credentials::UserAuthToken {
            token: config.token.clone(),
        };

        Ok(Self {
            api_client: CloudflareClient::new(
                credentials,
                CloudflareConfig::default(),
                Environment::Production,
            )?,
            config,
            cached_ip4: Ipv4Addr::new(0, 0, 0, 0),
            updated_last_time: DateTime::default(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.verify_token().await?;

        info!("cache TTL: {}", format_duration(&self.config.cache_ttl));
        info!("machine identifier: {}", self.config.machine_id);
        info!("update cron: {}", self.config.update_cron);

        loop {
            let now = Local::now();
            let next = self.config.update_cron.find_next_occurrence(&now, false)?;

            let ip4 = match get_ip4().await {
                Ok(ipv4) => ipv4,
                Err(e) => {
                    error!("failed to fetch ipv4 address! waiting 2 minutes for next check...");
                    sleep(Duration::from_mins(2)).await;
                    continue;
                }
            };

            let ip_changed = ip4 != self.cached_ip4;
            let cache_expired = now - self.updated_last_time >= self.config.cache_ttl;

            if ip_changed || cache_expired {
                tracing::info!("updating with the IP: {}", ip4);
                tracing::debug!("next scheduled update: {}", next);

                self.cached_ip4 = ip4;

                self.update_records().await?;

                self.updated_last_time = now;
            }

            sleep((next - now).to_std()?).await;
        }
    }

    async fn verify_token(&self) -> Result<()> {
        let endpoint = GetUserTokenStatus {};
        let res = self.api_client.request(&endpoint).await;

        match res {
            Ok(status) => match status.result.status.as_str() {
                "active" => {
                    debug!("successfully verified API token");
                    Ok(())
                }
                "disabled" => Err(AppError::InvalidToken("disabled".into()))?,
                "expired" => Err(AppError::InvalidToken("expired".into()))?,
                _ => todo!(),
            },
            Err(e) => bail!("failed to verify api token: {:?}", e),
        }
    }

    async fn update_records(&self) -> Result<()> {
        // list all zones that we have access to
        let zones = {
            let res = self
                .api_client
                .request(&ListZones {
                    params: ListZonesParams {
                        name: None,
                        status: None,
                        page: None,
                        per_page: Some(50),
                        order: Some(ListZonesOrder::Name),
                        direction: Some(OrderDirection::Ascending),
                        search_match: None,
                    },
                })
                .await;
            match res {
                Ok(r) => r.result,
                _ => todo!(),
            }
        };

        for zone in zones {
            let records = {
                let res = self
                    .api_client
                    .request(&ListDnsRecords {
                        zone_identifier: &zone.id,
                        params: ListDnsRecordsParams {
                            record_type: None,
                            name: None,
                            page: None,
                            per_page: Some(500),
                            order: Some(ListDnsRecordsOrder::Name),
                            direction: Some(OrderDirection::Ascending),
                            search_match: None,
                        },
                    })
                    .await;
                match res {
                    Ok(r) => r.result,
                    _ => todo!(),
                }
            };

            trace!(
                "fetched records [ {} ]",
                records
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            );

            for record in records {
                if !self.config.machine_id.is_empty()
                    && record
                        .comment
                        .clone()
                        .unwrap_or("".to_string())
                        .contains(format!("DDNS_ID={}", &self.config.machine_id).as_str())
                {
                    // Update the record

                    let res = self
                        .api_client
                        .request(&UpdateDnsRecord {
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
                        })
                        .await;
                    match res {
                        Ok(_) => {
                            debug!("updated record {} -> {}", record.name, self.cached_ip4);
                        }
                        Err(e) => panic!("failed to update record: {}", e),
                    }
                }
            }
        }

        Ok(())
    }
}
