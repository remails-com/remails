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
    #[error("could not generate signature: {0}")]
    DkimError(mail_auth::Error),
    #[error("invalid domain: {0}")]
    DomainError(&'static str),
    // TODO: a message can be held for more than one reason
    #[error("message is being held: DKIM not in DNS")]
    MessageHeld,
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

    async fn check_dkim_key(&self, domain_key: &PrivateKey<'_>, domain: &str) -> Option<()> {
        let domain = domain.trim_matches('.');

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

        if dns_key.iter().eq(domain_key.public_key()) {
            Some(())
        } else {
            trace!("dkim keys are not equal!");
            None
        }
    }

    async fn check_message(
        &self,
        message: &Message,
        parsed_msg: &mail_parser::Message<'_>,
        domain: &str,
        key: &PrivateKey<'_>,
    ) -> MessageStatus {
        // check MAIL FROM domain
        let sender_domain = message.from_email.domain();
        if sender_domain != domain {
            info!("message held due to MAIL FROM domain ({sender_domain}) != {domain}");
            return MessageStatus::Held;
        }

        // check From domain
        if let Some(from) = parsed_msg.from() {
            for addr in from.iter() {
                if let Some(Ok(addr)) = addr.address().map(|p| p.parse::<EmailAddress>()) {
                    if addr.domain() != domain {
                        info!(
                            "message held due to From domain ({}) != {domain}",
                            addr.domain()
                        );
                        return MessageStatus::Held;
                    }
                }
            }
        };

        // check Return-Path domain
        if let Some(Ok(return_path)) = parsed_msg
            .return_address()
            .map(|p| p.parse::<EmailAddress>())
        {
            if return_path.domain() != domain {
                info!(
                    "message held due to Return-Path domain ({}) != {domain}",
                    return_path.domain()
                );
                return MessageStatus::Held;
            }
        };

        // check dkim key
        if self.check_dkim_key(key, sender_domain).await.is_none() {
            info!("message held due to invalid DKIM key");
            return MessageStatus::Held;
        }

        MessageStatus::Accepted
    }

    pub async fn handle_message(&self, message: NewMessage) -> Result<Message, HandlerError> {
        let mut message: Message = self
            .message_repository
            .create(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        trace!("stored message {}", message.id());

        // retrieve the dkim key from the database
        let domain = self
            .domain_repository
            .get(
                message.organization_id,
                Some(message.project_id),
                message
                    .domain_id
                    .ok_or(HandlerError::DomainError("no domain ID in message"))?,
            )
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        let key =
            PrivateKey::new(&domain, "remails").map_err(HandlerError::MessageRepositoryError)?;

        trace!(
            "retrieved dkim key for domain {}: {}",
            domain.domain,
            Base64::encode_string(key.public_key())
        );
        trace!("parsing message {} {}", message.id(), message.message_data);

        // parse and save message contents
        let parsed_msg: mail_parser::Message = MessageParser::default()
            .parse(&message.raw_data)
            .unwrap_or_else(|| mail_parser::Message {
                raw_message: Borrowed(&message.raw_data),
                ..Default::default()
            });

        // this should never fail since mail_parser::Message has a derived Serialize instance
        let json_message_data =
            serde_json::to_value(&parsed_msg).map_err(HandlerError::SerializeMessageData)?;

        trace!("updating message {}", message.id());

        message.message_data = json_message_data;

        message.status = self
            .check_message(&message, &parsed_msg, &domain.domain, &key)
            .await;

        self.message_repository
            .update_message_data(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        if message.status != MessageStatus::Accepted {
            return Err(HandlerError::MessageHeld);
        }

        // generate message headers

        let mut generated_headers = String::new();

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

            generated_headers.push_str(&format!(
                "Message-ID: <REMAILS-{hash}@{}>\r\n",
                domain.domain
            ));
        }

        // sign with dkim

        trace!("signing with dkim");

        generated_headers.push_str(
            &key.dkim_header(&parsed_msg)
                .map_err(HandlerError::DkimError)?,
        );

        trace!("adding headers");
        debug!("{generated_headers:?}");

        // TODO: we could 'overallocate' the original raw message data to prepend this stuff without
        // needing to allocate or move data around.
        let hdr_size = generated_headers.len();
        let msg_len = message.raw_data.len();

        message
            .raw_data
            .resize(msg_len + hdr_size, Default::default());
        message.raw_data.copy_within(..msg_len, hdr_size);
        message.raw_data[..hdr_size].copy_from_slice(generated_headers.as_bytes());

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
}
