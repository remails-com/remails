pub use crate::handler::connection_log::ConnectionLog;
use crate::{
    Environment,
    bus::client::{BusClient, BusMessage},
    dkim::PrivateKey,
    handler::{
        connection_log::LogLevel,
        dns::{DnsResolver, ResolveError},
    },
    kubernetes::Kubernetes,
    models::{
        DeliveryStatus, DomainRepository, Message, MessageId, MessageRepository, MessageStatus,
        OrganizationRepository, QuotaStatus,
    },
};
use base64ct::{Base64, Encoding};
use chrono::Duration;
use email_address::EmailAddress;
use futures::StreamExt;
use mail_parser::{HeaderName, MessageParser};
use mail_send::{SmtpClient, SmtpClientBuilder, smtp};
use sqlx::PgPool;
use std::{borrow::Cow::Borrowed, collections::BTreeSet, fmt::Display, net::IpAddr, sync::Arc};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Semaphore,
    task::JoinHandle,
};
use tokio_rustls::rustls::{crypto, crypto::CryptoProvider};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

mod connection_log;

pub mod dns;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("DB interaction failed: {0}")]
    RepositoryError(#[from] crate::models::Error),
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
    #[error("message is being {0:?}: {1}")]
    MessageNotAccepted(MessageStatus, String),
    #[error("Message is in an illegal state: {0}, {1:?}")]
    IllegalMessageState(MessageStatus, MessageId),
    #[error("Message must have an outbound IP assigned")]
    MissingOutboundIp,
}

#[derive(Debug, Error)]
enum SendError {
    #[error("could not find a working MX receiver")]
    PermanentFailure,
    #[error("no MX server accepted the message")]
    TemporaryFailure,
}

#[derive(Clone, Copy)]
enum Protection {
    Plaintext,
    Tls,
}

#[derive(Clone)]
pub struct RetryConfig {
    pub(crate) delay: Duration,
    pub(crate) max_automatic_retries: i32,
}

impl RetryConfig {
    pub fn new() -> Self {
        Self {
            delay: Duration::minutes(5),
            max_automatic_retries: 5,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct HandlerConfig {
    pub(crate) resolver: DnsResolver,
    pub(crate) domain: String,
    pub(crate) allow_plain: bool,
    pub(crate) retry: RetryConfig,
    pub(crate) environment: Environment,
}

#[cfg(not(test))]
impl HandlerConfig {
    pub fn new() -> Self {
        Self {
            allow_plain: false,
            domain: std::env::var("SMTP_EHLO_DOMAIN")
                .expect("Missing SMTP_EHLO_DOMAIN environment variable"),
            resolver: DnsResolver::new(),
            retry: Default::default(),
            environment: Environment::from_env(),
        }
    }
}

#[cfg(not(test))]
impl Default for HandlerConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Handler {
    message_repository: MessageRepository,
    domain_repository: DomainRepository,
    organization_repository: OrganizationRepository,
    k8s: Kubernetes,
    workers: Arc<Semaphore>,
    bus_client: BusClient,
    outbound_ips: BTreeSet<IpAddr>,
    shutdown: CancellationToken,
    config: Arc<HandlerConfig>,
}

impl Handler {
    pub async fn new(
        pool: PgPool,
        config: Arc<HandlerConfig>,
        bus_client: BusClient,
        shutdown: CancellationToken,
    ) -> Self {
        if CryptoProvider::get_default().is_none() {
            CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
                .expect("Failed to install crypto provider");
        }

        Self {
            message_repository: MessageRepository::new(pool.clone()),
            domain_repository: DomainRepository::new(pool.clone()),
            organization_repository: OrganizationRepository::new(pool.clone()),
            k8s: Kubernetes::new(pool.clone())
                .await
                .expect("Failed to initialize Kubernetes"),
            workers: Arc::new(Semaphore::new(100)),
            bus_client,
            outbound_ips: Default::default(),
            shutdown,
            config,
        }
    }

    fn is_valid_domain(domain: &str) -> bool {
        // RFC 1035: domains can only contain a-z, A-Z, 0-9, '-', and '.'
        // This should specifically prevent characters like '/', '?', and '#' from being used to extend domain names
        // E.g. "tweedegolf.com?q=gmail.com" is NOT allowed
        domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    }

    fn is_subdomain(subdomain: &str, domain: &str) -> bool {
        if !Self::is_valid_domain(domain) {
            return false;
        }

        if !Self::is_valid_domain(subdomain) {
            return false;
        }

        domain.ends_with(subdomain)
    }

    /// Check if we are able to send this message, i.e., we are permitted to use the sender's domain,
    /// and then we sign the message with DKIM
    ///
    /// # Returns
    /// * `Ok(Ok(dkim_header))` if all checks passed and we successfully signed the message
    /// * `Ok(Err((status, reason)))` when a message should be held or rejected for some reason
    /// * `Err(handler_error)` on critical internal server errors (mostly related to the database)
    async fn check_and_sign_message(
        &self,
        message: &Message,
        parsed_msg: &mail_parser::Message<'_>,
    ) -> Result<Result<String, (MessageStatus, String)>, HandlerError> {
        let sender_domain = message.from_email.domain();

        let Some(domain) = self
            .domain_repository
            .lookup_domain_name(sender_domain, message.project_id)
            .await
            .map_err(HandlerError::RepositoryError)?
        else {
            return Ok(Err((
                MessageStatus::Held,
                format!("Project is not permitted to use domain {sender_domain}"),
            )));
        };

        // check MAIL FROM domain (can be a subdomain)
        if !Self::is_subdomain(sender_domain, &domain.domain) {
            return Ok(Err((
                MessageStatus::Rejected,
                format!(
                    "MAIL FROM domain ({sender_domain}) is not a valid (sub-)domain of {}",
                    domain.domain
                ),
            )));
        }

        // check From domain (can be a different subdomain)
        if let Some(from) = parsed_msg.from() {
            for addr in from.iter() {
                if let Some(Ok(addr)) = addr.address().map(|p| p.parse::<EmailAddress>())
                    && !Self::is_subdomain(addr.domain(), &domain.domain)
                {
                    return Ok(Err((
                        MessageStatus::Rejected,
                        format!(
                            "From domain ({}) is not a valid (sub-)domain of {}",
                            addr.domain(),
                            domain.domain
                        ),
                    )));
                }
            }
        };

        // check Return-Path domain (can be a different subdomain)
        if let Some(Ok(return_path)) = parsed_msg
            .return_address()
            .map(|p| p.parse::<EmailAddress>())
            && !Self::is_subdomain(return_path.domain(), &domain.domain)
        {
            return Ok(Err((
                MessageStatus::Rejected,
                format!(
                    "Return-Path domain ({}) is not a valid (sub-)domain of {}",
                    return_path.domain(),
                    domain.domain
                ),
            )));
        };

        // check dkim key
        let dkim_key = match PrivateKey::new(&domain, &self.config.resolver.dkim_selector) {
            Ok(key) => key,
            Err(e) => {
                error!("error creating DKIM key: {e}");
                return Ok(Err((
                    MessageStatus::Held,
                    "internal error: could not create DKIM key".to_string(),
                )));
            }
        };
        trace!(
            "retrieved dkim key for domain {}: {}",
            domain.domain,
            Base64::encode_string(dkim_key.public_key())
        );
        if let Err(reason) = self
            .config
            .resolver
            .verify_dkim(sender_domain, dkim_key.public_key())
            .await
        {
            return Ok(Err((
                MessageStatus::Held,
                format!("invalid DKIM on {sender_domain}: {reason}"),
            )));
        }

        trace!("signing with dkim");
        let dkim_header = match dkim_key.dkim_header(parsed_msg) {
            Ok(header) => header,
            Err(e) => {
                error!("error creating DKIM header: {e}");
                return Ok(Err((
                    MessageStatus::Held,
                    "internal error: could not create DKIM header".to_string(),
                )));
            }
        };

        // The quota check needs to be the very last check,
        // as otherwise we might count messages that are held towards the quota.
        // Additionally,
        // we should only deduce the quota for messages
        // that are new and have not been counted to the quota before,
        // i.e., only messages in "Processing" and "Held" state.
        #[allow(clippy::collapsible_if)]
        if matches!(
            message.status,
            MessageStatus::Processing | MessageStatus::Held
        ) {
            if QuotaStatus::Exceeded
                == self
                    .organization_repository
                    .reduce_quota(message.organization_id)
                    .await?
            {
                return Ok(Err((MessageStatus::Held, "Quota exceeded".to_string())));
            }
        }

        Ok(Ok(dkim_header))
    }

    pub async fn handle_message(&self, message: &mut Message) -> Result<(), HandlerError> {
        fn parse_message<'a>(raw_data: &'a Vec<u8>) -> mail_parser::Message<'a> {
            MessageParser::default()
                .parse(raw_data)
                .unwrap_or_else(|| mail_parser::Message {
                    raw_message: Borrowed(raw_data),
                    ..Default::default()
                })
        }

        // parse, add new headers if needed, and save message contents
        let mut parsed_msg: mail_parser::Message = parse_message(&message.raw_data);

        let has_header = |name: HeaderName| {
            parsed_msg
                .parts
                .first()
                .is_some_and(|msg| msg.headers.iter().any(|hdr| hdr.name == name))
        };

        let mut new_headers = Vec::new();

        if !has_header(HeaderName::MessageId) {
            let message_id_header =
                MessageRepository::generate_message_id_header(&message.id(), &message.from_email);
            trace!("adding Message-ID header: {message_id_header}");

            new_headers.push(format!("Message-ID: <{message_id_header}>\r\n"));
            message.message_id_header = Some(message_id_header);
        }

        if !has_header(HeaderName::Date) {
            trace!("adding Date header");
            let date = chrono::Utc::now().to_rfc2822();
            new_headers.push(format!("Date: {date}\r\n"));
        }

        if !new_headers.is_empty() {
            trace!("updating message {}", message.id());
            message.prepend_headers(&new_headers.join(""));

            // we need to re-parse the message because the data has shifted
            parsed_msg = parse_message(&message.raw_data);
        }

        message.message_data =
            serde_json::to_value(&parsed_msg).map_err(HandlerError::SerializeMessageData)?;

        let result = self.check_and_sign_message(message, &parsed_msg).await?;
        match result {
            Ok(_) => match &message.status {
                // For messages being sent for the first time, update message status
                MessageStatus::Processing | MessageStatus::Held => {
                    message.status = MessageStatus::Accepted;
                }
                // For messages that have been processed before, keep the status as is
                MessageStatus::Reattempt
                | MessageStatus::Failed
                | MessageStatus::Accepted
                | MessageStatus::Rejected => {}
                // Other messages should not be processed (but we do want to save the message if this happens)
                MessageStatus::Delivered => {
                    error!(
                        message_id = message.id().to_string(),
                        "Delivered message should not be processed"
                    );
                    return Err(HandlerError::IllegalMessageState(
                        MessageStatus::Delivered,
                        message.id(),
                    ));
                }
            },
            Err((ref status, _)) => message.status = status.clone(),
        };
        message.reason = result.as_ref().err().map(|e| e.1.clone());

        message.set_next_retry(&self.config.retry);

        self.message_repository
            .update_message_data_and_status(message)
            .await
            .map_err(HandlerError::RepositoryError)?;

        let dkim_header = match result {
            Ok(dkim_header) => dkim_header,
            Err((status, reason)) => return Err(HandlerError::MessageNotAccepted(status, reason)),
        };

        trace!("adding DKIM header");
        trace!("{dkim_header:?}");
        message.prepend_headers(&dkim_header);

        Ok(())
    }

    async fn quit_smtp<T, D>(client: SmtpClient<T>, hostname: D)
    where
        D: Display,
        T: AsyncRead + AsyncWrite + Unpin,
    {
        client
            .quit()
            .await
            .inspect_err(|err| {
                warn!(
                    "failed to close upstream SMTP connection with {}: {err}",
                    hostname
                );
            })
            .ok();
    }

    async fn send_single_message(
        &self,
        recipient: &EmailAddress,
        message: smtp::message::Message<'_>,
        security: Protection,
        outbound_ip: IpAddr,
        connection_log: &mut ConnectionLog,
    ) -> Result<(), SendError> {
        let domain = recipient.domain();

        let mut priority = 0..65536;

        let mut is_temporary_failure = false;

        loop {
            match self
                .config
                .resolver
                .resolve_mail_domain(domain, &mut priority)
                .await
            {
                Ok((hostname, port)) => {
                    match self
                        .send_single_upstream(
                            security,
                            connection_log,
                            domain,
                            message.clone(),
                            &hostname,
                            port,
                            outbound_ip,
                        )
                        .await
                    {
                        Ok(_) => return Ok(()),
                        Err(SendError::PermanentFailure) => {} // continue to try the next server
                        Err(SendError::TemporaryFailure) => is_temporary_failure = true,
                    }
                }
                Err(ResolveError::AllServersExhausted) => {
                    info!(domain, "all mail servers exhausted");
                    connection_log.log(
                        LogLevel::Info,
                        format!("all mail servers for domain {domain} exhausted"),
                    );
                    break;
                }
                Err(ResolveError::Dns(err)) => {
                    error!(domain, "could not resolve mail domain: {err}");
                    connection_log.log(
                        LogLevel::Error,
                        format!("could not resolve domain '{domain}': {err}"),
                    );
                    is_temporary_failure = true;
                    break;
                }
            }
        }

        if is_temporary_failure {
            Err(SendError::TemporaryFailure)
        } else {
            Err(SendError::PermanentFailure)
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_single_upstream(
        &self,
        security: Protection,
        connection_log: &mut ConnectionLog,
        domain: &str,
        message: smtp::message::Message<'_>,
        hostname: &String,
        port: u16,
        outbound_ip: IpAddr,
    ) -> Result<(), SendError> {
        let smtp = SmtpClientBuilder::new(&hostname, port)
            .implicit_tls(false)
            .local_ip(outbound_ip)
            .say_ehlo(true)
            .helo_host(&self.config.domain)
            .timeout(std::time::Duration::from_secs(60));

        let result =
            match security {
                Protection::Tls => match smtp.connect().await {
                    Err(err) => Err(err),
                    Ok(mut client) => {
                        trace!(domain, port, "securely connected to upstream server");
                        connection_log.log(LogLevel::Info, format!(
                        "securely connected to '{hostname}' with port {port} over TLS",
                    ));
                        let result = client.send(message.clone()).await;
                        Self::quit_smtp(client, &hostname).await;
                        result
                    }
                },
                Protection::Plaintext => {
                    match smtp.connect_plain().await {
                        Err(err) => Err(err),
                        Ok(mut client) => {
                            trace!(domain, port, "INSECURELY connected to upstream server");
                            connection_log.log(LogLevel::Info, format!(
                            "INSECURELY connected to '{hostname}' with port {port} without TLS",
                        ));
                            let result = client.send(message.clone()).await;
                            Self::quit_smtp(client, &hostname).await;
                            result
                        }
                    }
                }
            };

        let Err(err) = result else {
            debug!(domain, port, "successfully send email");
            connection_log.log(
                LogLevel::Info,
                format!("successfully sent email using hostname '{hostname}' and port {port}",),
            );
            return Ok(());
        };

        info!(domain, port, "could not use server: {err}");
        connection_log.log(
            LogLevel::Warn,
            format!("could not use {hostname} on port {port}: {err}",),
        );

        Err(match err {
            mail_send::Error::Io(_) => SendError::TemporaryFailure,
            mail_send::Error::Tls(_) => SendError::TemporaryFailure,
            mail_send::Error::Base64(_) => SendError::TemporaryFailure,
            mail_send::Error::Auth(_) => SendError::TemporaryFailure,
            mail_send::Error::UnparseableReply => SendError::TemporaryFailure,
            mail_send::Error::UnexpectedReply(response)
            | mail_send::Error::AuthenticationFailed(response) => {
                // SMTP 4XX errors are temporary failures
                if response.severity() == smtp_proto::Severity::TransientNegativeCompletion {
                    SendError::TemporaryFailure
                } else {
                    SendError::PermanentFailure
                }
            }
            mail_send::Error::InvalidTLSName => SendError::TemporaryFailure,
            mail_send::Error::MissingCredentials => SendError::PermanentFailure,
            mail_send::Error::MissingMailFrom => SendError::PermanentFailure,
            mail_send::Error::MissingRcptTo => SendError::PermanentFailure,
            mail_send::Error::UnsupportedAuthMechanism => SendError::PermanentFailure,
            mail_send::Error::Timeout => SendError::TemporaryFailure,
            mail_send::Error::MissingStartTls => SendError::PermanentFailure,
        })
    }

    #[tracing::instrument(
        skip(self, message),
        fields(
            message_id = message.id().to_string(),
            organization_id = message.organization_id.to_string(),
            stream_id = message.stream_id.to_string(),
            outbound_ip = outbound_ip.to_string(),
        ))]
    pub async fn send_message(
        &self,
        mut message: Message,
        outbound_ip: IpAddr,
    ) -> Result<(), HandlerError> {
        info!("sending message");
        let mut failures = 0u32;
        let mut should_reattempt = false;

        let order: &[Protection] = if self.config.allow_plain {
            &[Protection::Tls, Protection::Plaintext]
        } else {
            &[Protection::Tls]
        };

        'next_rcpt: for recipient in &message.recipients {
            let delivery_details = message
                .delivery_details
                .entry(recipient.clone())
                .or_default();
            let connection_log = &mut delivery_details.log;

            match delivery_details.status {
                DeliveryStatus::None | DeliveryStatus::Reattempt => {
                    connection_log.log(
                        LogLevel::Info,
                        format!(
                            "attempting to send email to {} (attempt {})",
                            recipient.email(),
                            message.attempts
                        ),
                    );
                } // attempt to (re-)send
                DeliveryStatus::Success { .. } => {
                    connection_log.log(
                        LogLevel::Info,
                        format!(
                            "skipping recipient {} as message was already successfully (attempt {})",
                            recipient.email(), message.attempts
                        ),
                    );
                    continue;
                }
                DeliveryStatus::Failed => {
                    connection_log.log(
                        LogLevel::Info,
                        format!(
                            "skipping recipient {} as remote reported a permanent failure (attempt {})",
                            recipient.email(), message.attempts
                        ),
                    );
                    failures += 1;
                    continue;
                }
            }

            let mut is_temporary_failure = false;

            for &protection in order {
                // restrict the recipients; this object is cheap to clone
                let smtp_message = smtp::message::Message {
                    mail_from: message.from_email.as_str().into(),
                    rcpt_to: vec![recipient.email().into()],
                    body: message.raw_data.as_slice().into(),
                };
                match self
                    .send_single_message(
                        recipient,
                        smtp_message,
                        protection,
                        outbound_ip,
                        connection_log,
                    )
                    .await
                {
                    Ok(()) => {
                        delivery_details.status = DeliveryStatus::Success {
                            delivered: chrono::Utc::now(),
                        };
                        continue 'next_rcpt;
                    }
                    Err(SendError::TemporaryFailure) => is_temporary_failure = true,
                    Err(SendError::PermanentFailure) => {}
                }
            }
            failures += 1;

            if is_temporary_failure {
                should_reattempt = true;
                delivery_details.status = DeliveryStatus::Reattempt;
            } else {
                delivery_details.status = DeliveryStatus::Failed;
            }
        }

        message.status = if failures == 0 {
            MessageStatus::Delivered
        } else if should_reattempt {
            MessageStatus::Reattempt
        } else {
            MessageStatus::Failed
        };

        message.reason = if failures > 0 {
            Some(format!(
                "failed to deliver to {failures} of {} recipients",
                message.delivery_details.len()
            ))
        } else {
            let delivery_time = chrono::Utc::now() - message.created_at;
            let hours = delivery_time.num_hours();
            let minutes = delivery_time.num_minutes() % 60;
            let seconds = delivery_time.num_seconds() % 60;
            let millis = delivery_time.subsec_millis();
            if hours > 0 {
                Some(format!("in {hours}:{minutes:02}:{seconds:02}.{millis:03}s"))
            } else if minutes > 0 {
                Some(format!("in {minutes}:{seconds:02}.{millis:03}s"))
            } else {
                Some(format!("in {seconds}.{millis:03}s"))
            }
        };

        message.set_next_retry(&self.config.retry);

        self.message_repository
            .update_message_status(&mut message)
            .await
            .map_err(HandlerError::RepositoryError)?;

        self.bus_client
            .try_send(&BusMessage::EmailDeliveryAttempted(
                message.id(),
                message.status,
            ))
            .await;

        Ok(())
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut bus_stream = self
                .bus_client
                .receive_auto_reconnect(std::time::Duration::from_secs(1));

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = self.shutdown.cancelled() => {
                        info!("shutting down message handler");
                        return;
                    }
                    _ = interval.tick() => {
                        trace!("reload network interfaces");
                        let new_ips = if_addrs::get_if_addrs()
                            .expect("Cannot retrieve host network interfaces")
                            .into_iter()
                            .map(|iface| iface.ip())
                            // For now, we only handle IPv4 addresses. Maybe v6 will follow later
                            .filter_map(|ip| {match ip {
                                IpAddr::V4(v4) => Some(v4),
                                IpAddr::V6(_) => None
                            }})
                            .filter(|ip|
                                (!ip.is_loopback() || cfg!(test)) &&
                                !ip.is_unspecified() &&
                                !ip.is_multicast() &&
                                !ip.is_broadcast() &&
                                !ip.is_documentation() &&
                                // only allow private IPs if we're in development mode
                                (!ip.is_private() || matches!(self.config.environment, Environment::Development))
                            )
                            .map(Into::into)
                            .collect();
                        if new_ips != self.outbound_ips {
                            self.outbound_ips = new_ips;
                            info!("new interface list: {:?}", self.outbound_ips);
                            match self.k8s.save_available_node_ips(self.outbound_ips.clone()).await {
                                Ok(_) => {},
                                Err(e) => {
                                    error!("failed to save available node IPs: {e}");
                                    error!("Shutting down message handler as sending IPs are out of sync");
                                    self.shutdown.cancel();
                                }
                            }
                        }
                    }
                    message = bus_stream.next() => {
                        match message {
                            None => {
                                error!("Bus stream ended, shutting down");
                                self.shutdown.cancel();
                            },
                            Some(BusMessage::EmailReadyToSend(id, outbound_ip)) => {
                                if self.outbound_ips.contains(&outbound_ip) {
                                    self.handle_ready_to_send(id, outbound_ip).await;
                                } else {
                                    trace!(
                                        message_id = id.to_string(),
                                        outbound_ip = outbound_ip.to_string(),
                                        "skipping message as it should not be send from this node"
                                    );
                                }
                            },
                            _ => {} // ignore other messages
                        }
                    }
                }
            }
        })
    }

    async fn handle_ready_to_send(&self, id: MessageId, outbound_ip: IpAddr) {
        info!("Ready to send {id}");

        let Ok(permit) = self.workers.clone().acquire_owned().await else {
            error!("failed to acquire worker semaphore permit, shutting down");
            self.shutdown.cancel();
            return;
        };
        let self_clone = self.clone();
        tokio::spawn(async move {
            let _p = permit;

            // retrieve message from database
            let mut message = match self_clone.message_repository.get(id).await {
                Ok(message) => message,
                Err(e) => {
                    error!("failed to get message: {e:?}");
                    return;
                }
            };

            message.attempts += 1;

            let message_id = message.id().to_string();
            if let Err(e) = self_clone.handle_message(&mut message).await {
                if let HandlerError::MessageNotAccepted(MessageStatus::Held, reason) = &e {
                    warn!(message_id, "Message held: {reason}")
                } else {
                    error!(message_id, "failed to handle message: {e:?}");
                }
                return;
            };

            if let Err(e) = self_clone.send_message(message, outbound_ip).await {
                error!(message_id, "failed to send message: {e:?}");
            }
        });
    }
}

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        handler::dns::DnsResolver,
        models::{NewMessage, SmtpCredentialRepository, SmtpCredentialRequest},
        test::{TestStreams, random_port},
    };
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use mailcrab::TestMailServerHandle;
    use std::net::Ipv4Addr;

    impl Handler {
        pub(crate) async fn test_handler(pool: PgPool, mailcrab_port: u16) -> Self {
            let config = HandlerConfig {
                allow_plain: true,
                domain: "test".to_string(),
                resolver: DnsResolver::mock("localhost", mailcrab_port),
                environment: Environment::Development,
                retry: RetryConfig {
                    delay: Duration::minutes(5),
                    max_automatic_retries: 1,
                },
            };
            Handler::new(
                pool,
                Arc::new(config),
                BusClient::new_from_env_var().unwrap(),
                CancellationToken::new(),
            )
            .await
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "streams",
            "k8s_nodes"
        )
    ))]
    async fn test_handle_message(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle { token, rx: _rx } =
            mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let message: mail_send::smtp::message::Message = MessageBuilder::new()
            .from(("John Doe", "john@test-org-1-project-1.com"))
            .to(vec![
                ("Jane Doe", "jane@test-org-1-project-1.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .into_message()
            .unwrap();

        let credential_request = SmtpCredentialRequest {
            username: "user".to_string(),
            description: "Test SMTP credential description".to_string(),
        };

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo
            .generate(org_id, project_id, stream_id, &credential_request)
            .await
            .unwrap();

        let message = NewMessage::from_builder_message(message, credential.id());
        let handler = Handler::test_handler(pool.clone(), mailcrab_port).await;

        let mut message = handler
            .message_repository
            .create(&message, 1)
            .await
            .unwrap();
        handler.handle_message(&mut message).await.unwrap();
        handler
            .send_message(message, "127.0.0.1".parse().unwrap())
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "streams",
            "k8s_nodes"
        )
    ))]
    async fn test_handle_invalid_mail_from(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle { token, rx: _rx } =
            mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let we_cant_use_these_emails = [
            "john@gmail.com",
            "john@gmail.com/test-org-1-project-1.com",
            "john@gmail.com?q=test-org-1-project-1.com",
            "john@gmail.com#test-org-1-project-1.com",
        ];
        for from_email in we_cant_use_these_emails {
            let message: mail_send::smtp::message::Message = MessageBuilder::new()
                .from(("John Doe", from_email))
                .to(vec![
                    ("Jane Doe", "jane@test-org-1-project-1.com"),
                    ("James Smith", "james@test.com"),
                ])
                .subject("Hi!")
                .html_body("<h1>Hello, world!</h1>")
                .text_body("Hello world!")
                .into_message()
                .unwrap();

            let credential_request = SmtpCredentialRequest {
                username: "user".to_string(),
                description: "Test SMTP credential description".to_string(),
            };

            let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

            let credential_repo = SmtpCredentialRepository::new(pool.clone());
            let credential = credential_repo
                .generate(org_id, project_id, stream_id, &credential_request)
                .await
                .unwrap();

            // Message has invalid "MAIL FROM" and invalid "From"
            let message = NewMessage::from_builder_message(message, credential.id());
            let handler = Handler::test_handler(pool.clone(), mailcrab_port).await;

            let mut message = handler
                .message_repository
                .create(&message, 1)
                .await
                .unwrap();
            assert!(handler.handle_message(&mut message).await.is_err());

            credential_repo
                .remove(org_id, project_id, stream_id, credential.id())
                .await
                .unwrap();
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "streams",
            "k8s_nodes"
        )
    ))]
    async fn test_handle_invalid_from(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle { token, rx: _rx } =
            mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let we_cant_use_these_emails = [
            "john@gmail.com",
            "john@gmail.com/test-org-1-project-1.com",
            "john@gmail.com?q=test-org-1-project-1.com",
            "john@gmail.com#test-org-1-project-1.com",
        ];
        for from_email in we_cant_use_these_emails {
            let message: mail_send::smtp::message::Message = MessageBuilder::new()
                .from(("John Doe", from_email))
                .to(vec![
                    ("Jane Doe", "jane@test-org-1-project-1.com"),
                    ("James Smith", "james@test.com"),
                ])
                .subject("Hi!")
                .html_body("<h1>Hello, world!</h1>")
                .text_body("Hello world!")
                .into_message()
                .unwrap();

            let credential_request = SmtpCredentialRequest {
                username: "user".to_string(),
                description: "Test SMTP credential description".to_string(),
            };

            let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

            let credential_repo = SmtpCredentialRepository::new(pool.clone());
            let credential = credential_repo
                .generate(org_id, project_id, stream_id, &credential_request)
                .await
                .unwrap();

            // Message has valid "MAIL FROM" and invalid "From"
            let message = NewMessage::from_builder_message_custom_from(
                message,
                credential.id(),
                "john@test-org-1-project-1.com",
            );
            let handler = Handler::test_handler(pool.clone(), mailcrab_port).await;

            let mut message = handler
                .message_repository
                .create(&message, 1)
                .await
                .unwrap();
            assert!(handler.handle_message(&mut message).await.is_err());

            credential_repo
                .remove(org_id, project_id, stream_id, credential.id())
                .await
                .unwrap();
        }
    }
}
