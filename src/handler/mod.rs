use crate::{
    dkim::PrivateKey,
    models::{DomainRepository, Message, MessageRepository, MessageStatus, NewMessage},
};
use base64ct::{Base64, Base64Unpadded, Base64UrlUnpadded, Encoding};
use email_address::EmailAddress;
#[cfg_attr(test, allow(unused_imports))]
use hickory_resolver::{Resolver, name_server::TokioConnectionProvider};
use mail_parser::{HeaderName, MessageParser};
use mail_send::{SmtpClientBuilder, smtp};
use sqlx::PgPool;
use std::{borrow::Cow::Borrowed, ops::Range, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use tokio_rustls::rustls::{crypto, crypto::CryptoProvider};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("failed to persist message: {0}")]
    MessageRepositoryError(crate::models::Error),
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
    #[error("message is being held: {0}")]
    MessageHeld(String),
}

#[derive(Debug, Error)]
enum SendError {
    #[error("could not find a working MX receiver")]
    NoWorkingMx,
}

//TODO: do we want to do anything with DNS errors?
enum ResolveError {
    #[allow(dead_code)]
    Dns(hickory_resolver::ResolveError),
    AllServersExhausted,
}

#[derive(Clone, Copy)]
enum Protection {
    Plaintext,
    Tls,
}

pub struct HandlerConfig {
    #[cfg(not(test))]
    pub(crate) resolver: Resolver<TokioConnectionProvider>,
    #[cfg(test)]
    pub(crate) resolver: mock::Resolver,
    pub(crate) domain: String,
    pub(crate) allow_plain: bool,
}

#[cfg(not(test))]
impl HandlerConfig {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            allow_plain: false,
            domain: domain.into(),
            resolver: Resolver::builder_tokio()
                .expect("could not build Resolver")
                .build(),
        }
    }

    pub fn allow_plain_smtp(mut self, value: bool) -> Self {
        self.allow_plain = value;
        self
    }
}

pub struct Handler {
    message_repository: MessageRepository,
    domain_repository: DomainRepository,
    shutdown: CancellationToken,
    config: Arc<HandlerConfig>,
}

impl Handler {
    pub fn new(pool: PgPool, config: Arc<HandlerConfig>, shutdown: CancellationToken) -> Self {
        if CryptoProvider::get_default().is_none() {
            CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
                .expect("Failed to install crypto provider");
        }
        Self {
            message_repository: MessageRepository::new(pool.clone()),
            domain_repository: DomainRepository::new(pool),
            shutdown,
            config,
        }
    }

    async fn check_dkim_key(&self, public_dkim_key: &[u8], sender_domain: &str) -> Option<()> {
        let domain = sender_domain.trim_matches('.');

        let record = format!(
            "remails._domainkey.{domain}{}",
            if domain.ends_with('.') { "" } else { "." }
        );

        trace!("requesting DKIM key {record}");

        let dkim_record = self.config.resolver.txt_lookup(record).await.ok()?;

        let dkim_record = String::from_utf8(
            dkim_record
                .into_iter()
                .next()?
                .txt_data()
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
        )
        .ok()?;

        trace!("dkim record: {dkim_record}");

        let dns_key = dkim_record
            .split(';')
            .filter_map(|field| field.trim().split_once('='))
            .find(|(key, _value)| *key == "p")?
            .1;

        let dns_key = Base64Unpadded::decode_vec(dns_key).ok()?;

        if dns_key.iter().eq(public_dkim_key) {
            Some(())
        } else {
            trace!("dkim keys are not equal!");
            None
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
    /// * `Ok(Err(reason))` when a message should be held in the database
    /// * `Err(handler_error)` on critical internal server errors (mostly related to the database)
    async fn check_and_sign_message(
        &self,
        message: &Message,
        parsed_msg: &mail_parser::Message<'_>,
    ) -> Result<Result<String, String>, HandlerError> {
        let sender_domain = message.from_email.domain();

        // check SMTP credentials
        let Some(smtp_credential_id) = message.smtp_credential_id else {
            return Ok(Err("missing SMTP credential".to_string()));
        };
        let Some(domain_id) = self
            .domain_repository
            .get_domain_id_associated_with_credential(sender_domain, smtp_credential_id)
            .await
            .map_err(HandlerError::MessageRepositoryError)?
        else {
            return Ok(Err(format!(
                "SMTP credential is not permitted to use domain {sender_domain}"
            )));
        };

        let domain = self
            .domain_repository
            .get_domain_by_id(message.organization_id, domain_id)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        // check MAIL FROM domain (can be a subdomain)
        if !Self::is_subdomain(sender_domain, &domain.domain) {
            return Ok(Err(format!(
                "MAIL FROM domain ({sender_domain}) is not a valid (sub-)domain of {}",
                domain.domain
            )));
        }

        // check From domain (can be a different subdomain)
        if let Some(from) = parsed_msg.from() {
            for addr in from.iter() {
                if let Some(Ok(addr)) = addr.address().map(|p| p.parse::<EmailAddress>()) {
                    if !Self::is_subdomain(addr.domain(), &domain.domain) {
                        return Ok(Err(format!(
                            "From domain ({}) is not a valid (sub-)domain of {}",
                            addr.domain(),
                            domain.domain
                        )));
                    }
                }
            }
        };

        // check Return-Path domain (can be a different subdomain)
        if let Some(Ok(return_path)) = parsed_msg
            .return_address()
            .map(|p| p.parse::<EmailAddress>())
        {
            if !Self::is_subdomain(return_path.domain(), &domain.domain) {
                return Ok(Err(format!(
                    "Return-Path domain ({}) is not a valid (sub-)domain of {}",
                    return_path.domain(),
                    domain.domain
                )));
            }
        };

        // check dkim key
        let dkim_key = match PrivateKey::new(&domain, "remails") {
            Ok(key) => key,
            Err(e) => {
                error!("error creating DKIM key: {e}");
                return Ok(Err("internal error: could not create DKIM key".to_string()));
            }
        };
        trace!(
            "retrieved dkim key for domain {}: {}",
            domain.domain,
            Base64::encode_string(dkim_key.public_key())
        );
        if self
            .check_dkim_key(dkim_key.public_key(), sender_domain)
            .await
            .is_none()
        {
            return Ok(Err(format!("invalid DKIM key found on {sender_domain}")));
        }

        trace!("signing with dkim");
        Ok(dkim_key
            .dkim_header(parsed_msg)
            .inspect_err(|e| error!("error creating DKIM header: {e}"))
            .map_err(|_| "internal error: could not create DKIM header".to_string()))
    }

    pub async fn handle_message(&self, message: NewMessage) -> Result<Message, HandlerError> {
        let mut message: Message = self
            .message_repository
            .create(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        trace!("stored message {}", message.id());

        // parse and save message contents
        let mut parsed_msg: mail_parser::Message = MessageParser::default()
            .parse(&message.raw_data)
            .unwrap_or_else(|| mail_parser::Message {
                raw_message: Borrowed(&message.raw_data),
                ..Default::default()
            });

        // this should never fail since mail_parser::Message has a derived Serialize instance
        let mut json_message_data =
            serde_json::to_value(&parsed_msg).map_err(HandlerError::SerializeMessageData)?;

        if !parsed_msg.parts.first().is_some_and(|msg| {
            msg.headers
                .iter()
                .any(|hdr| hdr.name == HeaderName::MessageId)
        }) {
            // the message-id header was not provided by the MUA, we are going to
            // provide one ourselves.
            trace!("adding message-id header");
            use aws_lc_rs::digest;
            let hash = digest::digest(&digest::SHA224, &message.raw_data);
            let hash = Base64UrlUnpadded::encode_string(hash.as_ref());

            let sender_domain = message.from_email.domain();
            message.prepend_headers(&format!("Message-ID: <REMAILS-{hash}@{sender_domain}>\r\n"));

            // we need to re-parse the message because the data has shifted
            parsed_msg = MessageParser::default()
                .parse(&message.raw_data)
                .unwrap_or_else(|| mail_parser::Message {
                    raw_message: Borrowed(&message.raw_data),
                    ..Default::default()
                });

            json_message_data =
                serde_json::to_value(&parsed_msg).map_err(HandlerError::SerializeMessageData)?;
        }

        trace!("updating message {}", message.id());

        message.message_data = json_message_data;

        let result = self.check_and_sign_message(&message, &parsed_msg).await?;
        message.status = match result {
            Ok(_) => MessageStatus::Accepted,
            Err(_) => MessageStatus::Held,
        };

        self.message_repository
            .update_message_data(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        let dkim_header = match result {
            Ok(dkim_header) => dkim_header,
            Err(reason) => return Err(HandlerError::MessageHeld(reason)),
        };

        // generate message headers

        trace!("adding headers");
        debug!("{dkim_header:?}");

        message.prepend_headers(&dkim_header);

        Ok(message)
    }

    async fn resolve_mail_domain(
        &self,
        domain: &str,
        prio: &mut Range<u32>,
    ) -> Result<(String, u16), ResolveError> {
        let smtp_port = 25;

        // from https://docs.rs/hickory-resolver/latest/hickory_resolver/struct.Resolver.html#method.mx_lookup:
        // "hint queries that end with a ‘.’ are fully qualified names and are cheaper lookups"
        let domain = format!("{domain}{}", if domain.ends_with('.') { "" } else { "." });

        let lookup = self
            .config
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

    async fn send_single_message(
        &self,
        recipient: &EmailAddress,
        message: &Message,
        security: Protection,
    ) -> Result<(), SendError> {
        let domain = recipient.domain();

        let mut priority = 0..65536;

        // restrict the recipients; this object is cheap to clone
        let message = smtp::message::Message {
            mail_from: message.from_email.as_str().into(),
            rcpt_to: vec![recipient.email().into()],
            body: message.raw_data.as_slice().into(),
        };

        while let Ok((hostname, port)) = self.resolve_mail_domain(domain, &mut priority).await {
            let smtp = SmtpClientBuilder::new(hostname, port)
                .implicit_tls(false)
                .say_ehlo(true)
                .helo_host(&self.config.domain)
                .timeout(Duration::from_secs(60));

            let result = match security {
                Protection::Tls => match smtp.connect().await {
                    Err(err) => Err(err),
                    Ok(mut client) => {
                        trace!("securely connected to upstream server");
                        client.send(message.clone()).await
                    }
                },
                Protection::Plaintext => match smtp.connect_plain().await {
                    Err(err) => Err(err),
                    Ok(mut client) => {
                        trace!("INSECURELY connected to upstream server");
                        client.send(message.clone()).await
                    }
                },
            };

            let Err(err) = result else { return Ok(()) };

            trace!("could not use server: {err}");
        }

        Err(SendError::NoWorkingMx)
    }

    pub async fn send_message(&self, mut message: Message) -> Result<(), HandlerError> {
        info!("sending message {}", message.id());
        let mut had_failures = true;

        let order: &[Protection] = if self.config.allow_plain {
            &[Protection::Tls, Protection::Plaintext]
        } else {
            &[Protection::Tls]
        };

        'next_rcpt: for recipient in &message.recipients {
            for &protection in order {
                // maybe we should take more interest in the content of these error messages?
                if self
                    .send_single_message(recipient, &message, protection)
                    .await
                    .is_ok()
                {
                    continue 'next_rcpt;
                }
            }
            had_failures = true;
        }

        self.message_repository
            .update_message_status(
                &mut message,
                if had_failures {
                    MessageStatus::Failed
                } else {
                    MessageStatus::Delivered
                },
            )
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        Ok(())
    }

    pub fn spawn(self, mut queue_receiver: Receiver<NewMessage>) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = self.shutdown.cancelled() => {
                        info!("shutting down message handler");
                        return;
                    }
                    queue_result = queue_receiver.recv() => {
                        let Some(message) = queue_result else {
                            error!("queue error, shutting down");
                            self.shutdown.cancel();
                            return
                        };

                        let parsed_message = match self.handle_message(message).await {
                            Ok(message) => message,
                            Err(e) => {
                                error!("failed to handle message: {e:?}");
                                continue
                            }
                        };

                        if let Err(e) = self.send_message(parsed_message).await {
                            error!("failed to send message: {e:?}");
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod test {
    use crate::models::{SmtpCredentialRepository, SmtpCredentialRequest};
    use std::net::Ipv4Addr;

    use super::*;

    use crate::test::random_port;
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use mailcrab::TestMailServerHandle;
    use serial_test::serial;
    use tracing_test::traced_test;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    #[traced_test]
    #[serial]
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

        let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo
            .generate(org_id, project_id, stream_id, &credential_request)
            .await
            .unwrap();

        let message = NewMessage::from_builder_message(message, credential.id());
        let config = HandlerConfig {
            allow_plain: true,
            domain: "test".to_string(),
            resolver: super::mock::Resolver("localhost", mailcrab_port),
        };
        let handler = Handler::new(pool, Arc::new(config), CancellationToken::new());

        let message = handler.handle_message(message).await.unwrap();
        handler.send_message(message).await.unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    #[traced_test]
    #[serial]
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

            let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
            let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
            let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

            let credential_repo = SmtpCredentialRepository::new(pool.clone());
            let credential = credential_repo
                .generate(org_id, project_id, stream_id, &credential_request)
                .await
                .unwrap();

            // Message has invalid "MAIL FROM" and invalid "From"
            let message = NewMessage::from_builder_message(message, credential.id());
            let config = HandlerConfig {
                allow_plain: true,
                domain: "test".to_string(),
                resolver: super::mock::Resolver("localhost", mailcrab_port),
            };
            let handler = Handler::new(pool.clone(), Arc::new(config), CancellationToken::new());

            assert!(handler.handle_message(message).await.is_err());

            credential_repo
                .remove(org_id, project_id, stream_id, credential.id())
                .await
                .unwrap();
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    #[traced_test]
    #[serial]
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

            let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
            let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
            let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

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
            let config = HandlerConfig {
                allow_plain: true,
                domain: "test".to_string(),
                resolver: super::mock::Resolver("localhost", mailcrab_port),
            };
            let handler = Handler::new(pool.clone(), Arc::new(config), CancellationToken::new());

            assert!(handler.handle_message(message).await.is_err());

            credential_repo
                .remove(org_id, project_id, stream_id, credential.id())
                .await
                .unwrap();
        }
    }
}
