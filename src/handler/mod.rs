use crate::models::{Message, MessageRepository, MessageStatus, NewMessage};
#[cfg_attr(test, allow(unused_imports))]
use hickory_resolver::{Resolver, name_server::TokioConnectionProvider};
use mail_parser::MessageParser;
use mail_send::{SmtpClientBuilder, smtp};
use sqlx::PgPool;
use std::{borrow::Cow::Borrowed, ops::Range, str::FromStr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use tokio_rustls::rustls::{crypto, crypto::CryptoProvider};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("failed to persist message: {0}")]
    MessageRepositoryError(crate::models::Error),
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
}

#[derive(Debug, Error)]
enum SendError {
    #[error("invalid recipient: {0}")]
    InvalidRcpt(email_address::Error),
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
            message_repository: MessageRepository::new(pool),
            shutdown,
            config,
        }
    }

    pub async fn handle_message(&self, message: NewMessage) -> Result<Message, HandlerError> {
        let mut message = self
            .message_repository
            .create(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        debug!("stored message {}", message.id());

        // TODO: check limits etc

        debug!("parsing message {} {}", message.id(), message.message_data);

        let json_message_data = {
            // parse and save message contents
            let message_data = MessageParser::default()
                .parse(&message.raw_data)
                .ok_or_else(|| mail_parser::Message {
                    raw_message: Borrowed(&message.raw_data),
                    ..Default::default()
                });

            // this should never fail since mail_parser::Message has a derived Serialize instance
            serde_json::to_value(&message_data).map_err(HandlerError::SerializeMessageData)?
        };

        debug!("updating message {}", message.id());

        message.message_data = json_message_data;

        self.message_repository
            .update_message_data(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

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
        recipient: &str,
        message: &Message,
        security: Protection,
    ) -> Result<(), SendError> {
        let mail_address = match email_address::EmailAddress::from_str(recipient) {
            Ok(address) => address,
            Err(err) => {
                warn!("Invalid email address {recipient}: {err}");
                return Err(SendError::InvalidRcpt(err));
            }
        };

        let domain = mail_address.domain();

        let mut priority = 0..65536;

        // restrict the recipients; this object is cheap to clone
        let message = smtp::message::Message {
            mail_from: message.from_email.as_str().into(),
            rcpt_to: vec![recipient.into()],
            body: message.raw_data.as_slice().into(),
        };

        while let Ok((hostname, port)) = self.resolve_mail_domain(domain, &mut priority).await {
            let smtp = SmtpClientBuilder::new(hostname, port)
                .implicit_tls(false)
                .say_ehlo(true)
                .helo_host(&self.config.domain)
                .timeout(Duration::from_secs(60));

            let result = match security {
                Protection::Tls => {
                    chain(smtp.connect().await, async |mut client| {
                        trace!("securely connected to upstream server");
                        client.send(message.clone()).await
                    })
                    .await
                }
                Protection::Plaintext => {
                    chain(smtp.connect_plain().await, async |mut client| {
                        trace!("INSECURELY connected to upstream server");
                        client.send(message.clone()).await
                    })
                    .await
                }
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

// Utility function similar to the 'and_then' function on Result
async fn chain<T1, T2, E>(
    result: Result<T1, E>,
    then: impl AsyncFnOnce(T1) -> Result<T2, E>,
) -> Result<T2, E> {
    let value = result?;
    then(value).await
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

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "domains")))]
    #[traced_test]
    #[serial]
    async fn test_handle_message(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle { token, rx: _rx } =
            mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let message: mail_send::smtp::message::Message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("Jane Doe", "jane@example.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .into_message()
            .unwrap();

        let credential_request = SmtpCredentialRequest {
            username: "user".to_string(),
            domain_id: "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        };

        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo.generate(&credential_request).await.unwrap();

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
