use std::ops::Range;

use base64ct::{Base64Unpadded, Encoding};
#[cfg(not(test))]
use hickory_resolver::{Resolver, name_server::TokioConnectionProvider};
use tracing::{debug, trace};

use crate::api::domains::VerifyResult;
#[cfg(test)]
use crate::handler::mock;

//TODO: do we want to do anything with DNS errors?
pub enum ResolveError {
    #[allow(dead_code)]
    Dns(hickory_resolver::ResolveError),
    AllServersExhausted,
}

#[derive(Clone)]
pub struct DnsResolver {
    #[cfg(not(test))]
    pub(crate) resolver: Resolver<TokioConnectionProvider>,
    #[cfg(test)]
    pub(crate) resolver: mock::Resolver,
    pub preferred_spf_record: String,
}

#[cfg(not(test))]
impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsResolver {
    #[cfg(not(test))]
    pub fn new() -> Self {
        Self {
            resolver: Resolver::builder_tokio()
                .expect("could not build Resolver")
                .build(),
            preferred_spf_record: std::env::var("PREFERRED_SPF_RECORD")
                .unwrap_or("v=spf1 include:spf.remails.net -all".to_string()),
        }
    }

    #[cfg(test)]
    pub fn mock(domain: &'static str, port: u16) -> Self {
        Self {
            resolver: mock::Resolver(domain, port),
            preferred_spf_record: "v=spf1 include:spf.remails.net -all".to_string(),
        }
    }

    pub async fn resolve_mail_domain(
        &self,
        domain: &str,
        prio: &mut Range<u32>,
    ) -> Result<(String, u16), ResolveError> {
        let smtp_port = 25;

        // from https://docs.rs/hickory-resolver/latest/hickory_resolver/struct.Resolver.html#method.mx_lookup:
        // "hint queries that end with a ‘.’ are fully qualified names and are cheaper lookups"
        let domain = format!("{domain}{}", if domain.ends_with('.') { "" } else { "." });

        let lookup = self
            .resolver
            .mx_lookup(&domain)
            .await
            .map_err(ResolveError::Dns)?;

        let Some(destination) = lookup
            .iter()
            .filter(|mx| prio.contains(&u32::from(mx.preference())))
            .min_by_key(|mx| mx.preference())
        else {
            return if prio.contains(&0) {
                prio.start = u32::MAX;
                Ok((domain, smtp_port))
            } else {
                Err(ResolveError::AllServersExhausted)
            };
        };

        #[cfg(test)]
        let smtp_port = destination.port();

        // make sure we don't accept this SMTP server again if it fails us
        prio.start = u32::from(destination.preference()) + 1;

        debug!("trying mail server: {destination:?}");
        Ok((destination.exchange().to_utf8(), smtp_port))
    }

    async fn get_singular_dns_record(
        &self,
        record: &str,
        starting_with: &str,
    ) -> Result<String, &'static str> {
        trace!("requesting DNS record {record}");
        let Ok(record) = self.resolver.txt_lookup(record).await else {
            return Err("could not retrieve DNS record");
        };

        let mut record = record.into_iter().filter(|r| {
            r.txt_data()
                .iter()
                .flatten()
                .take(starting_with.len())
                .eq(starting_with.as_bytes())
        });
        let Some(first_record) = record.next() else {
            return Err("record unavailable");
        };

        if let Some(_next_record) = record.next() {
            return Err("multiple conflicting DNS records available");
        }

        let data = first_record
            .txt_data()
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();

        String::from_utf8(data).or(Err("could not decode record"))
    }

    pub async fn verify_dkim(
        &self,
        domain: &str,
        dkim_pk_from_db: &[u8],
    ) -> Result<&'static str, &'static str> {
        let domain = domain.trim_matches('.');
        let record = format!("remails._domainkey.{domain}.");
        let dkim_data = self.get_singular_dns_record(&record, "v=DKIM1").await?;
        trace!("dkim data: {dkim_data:?}");

        let dns_key = dkim_data
            .split(';')
            .filter_map(|field| field.trim().split_once('='))
            .find(|(key, _value)| *key == "p")
            .ok_or("could not get public key from record")?
            .1;

        let Ok(dns_key) = Base64Unpadded::decode_vec(dns_key) else {
            return Err("could not decode DKIM key");
        };

        if dns_key.iter().eq(dkim_pk_from_db) {
            Ok("available!")
        } else {
            Err("public key in DNS record does not match")
        }
    }

    pub async fn verify_spf(&self, domain: &str) -> VerifyResult {
        let domain = domain.trim_matches('.');
        let record = format!("{domain}.");
        let spf_data = match self.get_singular_dns_record(&record, "v=spf1").await {
            Ok(spf_data) => spf_data,
            Err(reason) => return VerifyResult::error(reason),
        };
        trace!("spf data: {spf_data:?}");

        if spf_data == self.preferred_spf_record {
            VerifyResult::success("correct!")
        } else {
            VerifyResult::warning("currently configured differently:", Some(spf_data))
        }
    }

    pub async fn verify_dmarc(&self, domain: &str) -> VerifyResult {
        let domain = domain.trim_matches('.');
        let record = format!("_dmarc.{domain}.");
        let dmarc_data = match self.get_singular_dns_record(&record, "v=DMARC1").await {
            Ok(dmarc_data) => dmarc_data,
            Err(reason) => return VerifyResult::error(reason),
        };
        trace!("dmarc data: {dmarc_data:?}");

        if dmarc_data == "v=DMARC1; p=reject" {
            VerifyResult::success("correct!")
        } else {
            VerifyResult::warning("currently configured differently:", Some(dmarc_data))
        }
    }

    pub async fn any_a_record(&self, domain: &str) -> VerifyResult {
        let domain = format!("{}.", domain.trim_matches('.'));
        match self.resolver.lookup_ip(domain).await {
            Ok(ips) =>
            {
                #[cfg_attr(test, allow(clippy::iter_next_slice))]
                if ips.iter().next().is_some() {
                    VerifyResult::success("available")
                } else {
                    VerifyResult::warning("no A record set", None)
                }
            }
            Err(_) => VerifyResult::warning("could not retrieve DNS record", None),
        }
    }
}
