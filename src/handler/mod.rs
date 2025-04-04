use crate::models::{Message, MessageRepository, MessageStatus, NewMessage};
#[cfg_attr(test, allow(unused_imports))]
use hickory_resolver::{Resolver, name_server::TokioConnectionProvider};
use mail_parser::MessageParser;
use mail_send::SmtpClientBuilder;
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

//TODO: do we want to do anything with DNS errors?
enum ResolveError {
    #[allow(dead_code)]
    Dns(hickory_resolver::ResolveError),
    AllServersExhausted,
}

pub struct HandlerConfig {
    #[cfg(not(test))]
    pub resolver: Resolver<TokioConnectionProvider>,
    #[cfg(test)]
    pub resolver: mock::Resolver,
    pub domain: String,
}

#[cfg(not(test))]
impl HandlerConfig {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            resolver: Resolver::builder_tokio()
                .expect("could not build Resolver")
                .build(),
        }
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
        prio: &mut Range<u16>,
    ) -> Result<(String, u16), ResolveError> {
        let smtp_port = 25;

        // from https://docs.rs/hickory-resolver/latest/hickory_resolver/struct.Resolver.html#method.mx_lookup:
        // "hint queries that end with a ‘.’ are fully qualified names and are cheaper lookups"
        let domain = format!("{domain}.");

        let lookup = self
            .config
            .resolver
            .mx_lookup(&domain)
            .await
            .map_err(ResolveError::Dns)?;

        let Some(destination) = lookup
            .iter()
            .filter(|mx| prio.contains(&mx.preference()))
            .min_by_key(|mx| mx.preference())
        else {
            return if prio.contains(&0) {
                prio.start = u16::MAX;
                Ok((domain, smtp_port))
            } else {
                Err(ResolveError::AllServersExhausted)
            };
        };

        #[cfg(test)]
        let smtp_port = destination.port();

        // make sure we don't accept this SMTP server again if it fails us
        prio.start = destination.preference() + 1;

        debug!("using mail server: {destination:?}");
        Ok((destination.exchange().to_utf8(), smtp_port))
    }

    pub async fn send_message(&self, mut message: Message) -> Result<(), HandlerError> {
        info!("sending message {}", message.id());

        //TODO FIXME: this clone here isn't too bad, but maybe we can do better
        'rcpt: for recipient in &message.recipients.clone() {
            let mail_address = match email_address::EmailAddress::from_str(recipient) {
                Ok(address) => address,
                Err(err) => {
                    warn!("Invalid email address {recipient}: {err}");
                    continue 'rcpt;
                }
            };

            let domain = mail_address.domain();

            //TODO: only allow mx record if the preference is lower than our own, to prevent loops, see issue #74
            // we have here hardcoded our priority as being 1000
            let mut priority = 0..1000;

            'mx: while !priority.is_empty() {
                let Ok((hostname, port)) = self.resolve_mail_domain(domain, &mut priority).await
                else {
                    break 'mx;
                };

                let client = SmtpClientBuilder::new(hostname, port)
                    .implicit_tls(true)
                    .say_ehlo(true)
                    .helo_host(&self.config.domain)
                    .timeout(Duration::from_secs(60));

                let mut client = match client.connect().await {
                    Ok(client) => {
                        trace!("connected to upstream server");

                        client
                    }
                    Err(e) => {
                        error!("failed to connect to upstream server: {e}");

                        continue 'mx;
                    }
                };

                // TODO FIXME: since messages can be rather large, this clone can have a negative impact on the performance
                // if the server gets under stress; and it can probably be fixed by making "update_message_status" more efficient
                // (does it really need to UPDATE the message body simply for setting the status?)
                if let Err(e) = client.send(message.clone()).await {
                    error!("failed to send message: {e}");

                    continue 'mx;
                } else {
                    self.message_repository
                        .update_message_status(&mut message, MessageStatus::Delivered)
                        .await
                        .map_err(HandlerError::MessageRepositoryError)?;

                    break 'mx;
                }
            }

            self.message_repository
                .update_message_status(&mut message, MessageStatus::Failed)
                .await
                .map_err(HandlerError::MessageRepositoryError)?;

            // potential TODO: do we want to accumulate errors here to return at the end?
        }

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
            domain: "test".to_string(),
            resolver: super::mock::Resolver("localhost", mailcrab_port),
        };
        let handler = Handler::new(pool, Arc::new(config), CancellationToken::new());

        let message = handler.handle_message(message).await.unwrap();
        handler.send_message(message).await.unwrap();
    }
}
