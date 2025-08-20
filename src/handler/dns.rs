use base64ct::{Base64Unpadded, Encoding};
use chrono::{DateTime, Utc};
#[cfg(not(test))]
use hickory_resolver::{
    Resolver,
    config::{LookupIpStrategy::Ipv4Only, NameServerConfig, ResolverConfig, ResolverOpts},
    name_server::TokioConnectionProvider,
    proto::xfer::Protocol,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use tracing::{debug, trace};

#[cfg(test)]
use crate::handler::mock;
use crate::models::{Domain, Error};

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
    pub dkim_selector: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum VerifyResultStatus {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyResult {
    pub(crate) status: VerifyResultStatus,
    pub(crate) reason: String,
    pub(crate) value: Option<String>,
}

impl VerifyResult {
    pub fn error(reason: impl Into<String>, value: Option<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Error,
            reason: reason.into(),
            value,
        }
    }
    pub fn warning(reason: impl Into<String>, value: Option<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Warning,
            reason: reason.into(),
            value,
        }
    }
    pub fn info(reason: impl Into<String>, value: Option<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Info,
            reason: reason.into(),
            value,
        }
    }
    pub fn success(reason: impl Into<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Success,
            reason: reason.into(),
            value: None,
        }
    }
}

impl From<Result<&'static str, &'static str>> for VerifyResult {
    fn from(value: Result<&'static str, &'static str>) -> Self {
        VerifyResult {
            status: value
                .map(|_| VerifyResultStatus::Success)
                .unwrap_or(VerifyResultStatus::Error),
            reason: value.unwrap_or_else(|e| e).to_string(),
            value: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DomainVerificationStatus {
    timestamp: DateTime<Utc>,
    dkim: VerifyResult,
    spf: VerifyResult,
    dmarc: VerifyResult,
    a: VerifyResult,
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
        let mut resolver_options = ResolverOpts::default();
        // The cluster does not support DualStack
        resolver_options.ip_strategy = Ipv4Only;
        resolver_options.negative_max_ttl = Some(std::time::Duration::from_secs(20));
        resolver_options.attempts = 4;

        let mut resolver_config = ResolverConfig::new();
        // protective (DNS4EU)
        resolver_config.add_name_server(NameServerConfig {
            socket_addr: "86.54.11.1:853".parse().unwrap(),
            protocol: Protocol::Tls,
            tls_dns_name: Some("protective.joindns4.eu".to_string()),
            http_endpoint: None,
            trust_negative_responses: false,
            bind_addr: None,
        });
        resolver_config.add_name_server(NameServerConfig {
            socket_addr: "86.54.11.201:853".parse().unwrap(),
            protocol: Protocol::Tls,
            tls_dns_name: Some("protective.joindns4.eu".to_string()),
            http_endpoint: None,
            trust_negative_responses: false,
            bind_addr: None,
        });

        // Malware Blocking, DNSSEC Validation (Quad9)
        resolver_config.add_name_server(NameServerConfig {
            socket_addr: "9.9.9.9:853".parse().unwrap(),
            protocol: Protocol::Tls,
            tls_dns_name: Some("dns.quad9.net".to_string()),
            http_endpoint: None,
            trust_negative_responses: false,
            bind_addr: None,
        });
        resolver_config.add_name_server(NameServerConfig {
            socket_addr: "149.112.112.112:853".parse().unwrap(),
            protocol: Protocol::Tls,
            tls_dns_name: Some("dns.quad9.net".to_string()),
            http_endpoint: None,
            trust_negative_responses: false,
            bind_addr: None,
        });

        Self {
            resolver: Resolver::builder_with_config(
                resolver_config,
                TokioConnectionProvider::default(),
            )
            .with_options(resolver_options)
            .build(),
            dkim_selector: std::env::var("DKIM_SELECTOR")
                .expect("DKIM_SELECTOR environment variable not set"),
        }
    }

    #[cfg(test)]
    pub fn mock(domain: &'static str, port: u16) -> Self {
        Self {
            resolver: mock::Resolver {
                host: (domain, port),
                txt: "v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyQtyx8uwJIJoQ3+LEetDzd+bpIkebVIYSq94OCOimHu/Pv7tPY5pn99JVv0rmdGHluuWEGxQNBYDBdk0FQF4+HP0MlPitJSdxawmCRsIcUZR3TQLf6dDBm2YPJ3G4xUQ2pT4GPMwCX9N1aAfO5qj2fBsjT8LvLeTRKEbHXGDM+m2yMF0dgr6AJLLVYjs3MSD273DEL5GnqhGXieziz4PI5TCJpxR3CVByguImG9tg1BySMu3f7VFmiToLCVeuk1UzIYAPZN6fvCcmyalADfG9rZa/60lxFzeorBtVk/Ej0braeX8AT8RX2Ozw9lg2Wzkwx5NyvqOFAcnkhDX4oTeVQIDAQAB",
            },
            dkim_selector: "remails-testing".to_string(),
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
        let record = format!("{}._domainkey.{domain}.", self.dkim_selector);
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

    pub async fn verify_spf(&self, domain: &str, spf_include: &str) -> VerifyResult {
        let domain = domain.trim_matches('.');
        let record = format!("{domain}.");
        let spf_data = match self.get_singular_dns_record(&record, "v=spf1").await {
            Ok(spf_data) => spf_data,
            Err(reason) => return VerifyResult::error(reason, None),
        };
        trace!("spf data: {spf_data:?}");

        if spf_data == format!("v=spf1 {spf_include} -all") {
            return VerifyResult::success("correct!");
        }

        if !spf_data.split(' ').any(|x| x == spf_include) {
            return VerifyResult::warning(
                format!("SPF record is missing \"{spf_include}\":"),
                Some(spf_data),
            );
        }

        let last = spf_data.split(' ').next_back();
        if last != Some("-all") && last != Some("~all") {
            return VerifyResult::warning(
                "SPF record should end with -all (or ~all):",
                Some(spf_data),
            );
        }

        VerifyResult::info("currently configured as:", Some(spf_data))
    }

    pub async fn verify_dmarc(&self, domain: &str) -> VerifyResult {
        let domain = domain.trim_matches('.');
        let record = format!("_dmarc.{domain}.");
        let dmarc_data = match self.get_singular_dns_record(&record, "v=DMARC1").await {
            Ok(dmarc_data) => dmarc_data,
            Err(reason) => return VerifyResult::info(reason, None),
        };
        trace!("dmarc data: {dmarc_data:?}");

        // normalize dmarc record
        let dmarc_data = dmarc_data.trim_end_matches(";").replace("; ", ";");

        if dmarc_data == "v=DMARC1;p=reject" {
            VerifyResult::success("correct!")
        } else {
            VerifyResult::info("currently configured differently:", Some(dmarc_data))
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
                    VerifyResult::info("no A record set", None)
                }
            }
            Err(_) => VerifyResult::info("could not retrieve DNS record", None),
        }
    }

    pub async fn verify_domain(
        &self,
        domain: &Domain,
        spf_include: &str,
    ) -> Result<DomainVerificationStatus, Error> {
        Ok(DomainVerificationStatus {
            timestamp: Utc::now(),
            dkim: self
                .verify_dkim(&domain.domain, domain.dkim_key.pub_key()?.as_ref())
                .await
                .into(),
            spf: self.verify_spf(&domain.domain, spf_include).await,
            dmarc: self.verify_dmarc(&domain.domain).await,
            a: self.any_a_record(&domain.domain).await,
        })
    }
}

#[cfg(test)]
mod test {
    use sqlx::PgPool;

    use crate::models::DomainRepository;

    use super::*;

    #[tokio::test]
    async fn dkim_verification() {
        let domain = "localhost";
        let mut dns = DnsResolver::mock(domain, 0);

        let dkim_key = Base64Unpadded::decode_vec(
            "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyQtyx8uwJIJoQ3+LEetDzd+bpIkebVIYSq94OCOimHu/Pv7tPY5pn99JVv0rmdGHluuWEGxQNBYDBdk0FQF4+HP0MlPitJSdxawmCRsIcUZR3TQLf6dDBm2YPJ3G4xUQ2pT4GPMwCX9N1aAfO5qj2fBsjT8LvLeTRKEbHXGDM+m2yMF0dgr6AJLLVYjs3MSD273DEL5GnqhGXieziz4PI5TCJpxR3CVByguImG9tg1BySMu3f7VFmiToLCVeuk1UzIYAPZN6fvCcmyalADfG9rZa/60lxFzeorBtVk/Ej0braeX8AT8RX2Ozw9lg2Wzkwx5NyvqOFAcnkhDX4oTeVQIDAQAB"
        ).unwrap();

        dns.verify_dkim(domain, &dkim_key).await.unwrap();

        dns.resolver.txt = "v=DKIM1; k=rsa; p=wrongDkimKey";
        dns.verify_dkim(domain, &dkim_key)
            .await
            .expect_err("should error");
    }

    #[tokio::test]
    async fn spf_verification() {
        let domain = "localhost";
        let mut dns = DnsResolver::mock(domain, 0);

        dns.resolver.txt = ""; // spf record does not exist
        assert!(matches!(
            dns.verify_spf(domain, "include:test.com").await.status,
            VerifyResultStatus::Error
        ));

        dns.resolver.txt = "v=spf1 include:test.com -all";
        assert!(matches!(
            dns.verify_spf(domain, "include:test.com").await.status,
            VerifyResultStatus::Success
        ));

        dns.resolver.txt = "v=spf1 include:test.com include:spf.remails.com ~all";
        assert!(matches!(
            dns.verify_spf(domain, "include:test.com").await.status,
            VerifyResultStatus::Info
        ));

        dns.resolver.txt = "v=spf1 include:spf.remails.com -all";
        assert!(matches!(
            dns.verify_spf(domain, "include:test.com").await.status,
            VerifyResultStatus::Warning
        ));

        dns.resolver.txt = "v=spf1 include:test.com +all";
        assert!(matches!(
            dns.verify_spf(domain, "include:test.com").await.status,
            VerifyResultStatus::Warning
        ));
    }

    #[tokio::test]
    async fn dmarc_verification() {
        let domain = "localhost";
        let mut dns = DnsResolver::mock(domain, 0);
        dns.resolver.txt = ""; // dmarc record does not exist
        assert!(matches!(
            dns.verify_dmarc(domain).await.status,
            VerifyResultStatus::Info
        ));

        dns.resolver.txt = "v=DMARC1; p=reject";
        assert!(matches!(
            dns.verify_dmarc(domain).await.status,
            VerifyResultStatus::Success
        ));

        dns.resolver.txt = "v=DMARC1; p=reject;";
        assert!(matches!(
            dns.verify_dmarc(domain).await.status,
            VerifyResultStatus::Success
        ));

        dns.resolver.txt = "v=DMARC1;p=reject";
        assert!(matches!(
            dns.verify_dmarc(domain).await.status,
            VerifyResultStatus::Success
        ));

        dns.resolver.txt = "v=DMARC1;p=reject;sp=reject;adkim=s;aspf=s";
        assert!(matches!(
            dns.verify_dmarc(domain).await.status,
            VerifyResultStatus::Info
        ));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains", "streams")
    ))]
    async fn domain_verification(pool: PgPool) {
        let domains = DomainRepository::new(pool);
        let domain = domains
            .get_domain_by_id(
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        let dns = DnsResolver::mock("test-org-1.com", 0);

        let res = dns
            .verify_domain(&domain, "include:test.com")
            .await
            .unwrap();

        // The mock DNS resolver only contains the DKIM record
        assert!(matches!(res.dkim.status, VerifyResultStatus::Success));
        assert!(matches!(res.spf.status, VerifyResultStatus::Error));
        assert!(matches!(res.dmarc.status, VerifyResultStatus::Info));
        assert!(matches!(res.a.status, VerifyResultStatus::Success));
    }
}
