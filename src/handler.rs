use crate::models::{Message, MessageRepository, MessageStatus, NewMessage};
use mail_parser::MessageParser;
use mail_send::SmtpClientBuilder;
use sqlx::PgPool;
use std::{borrow::Cow::Borrowed, str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use tokio_rustls::rustls::{crypto, crypto::CryptoProvider};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};
use url::Url;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("failed to persist message: {0}")]
    MessageRepositoryError(crate::models::Error),
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
    #[error("failed to connect to upstream server: {0}")]
    ConnectToUpstream(mail_send::Error),
    #[error("failed to deliver message: {0}")]
    DeliverMessage(mail_send::Error),
}

pub struct HandlerConfig {
    pub test_smtp_addr: Option<Url>,
}

impl Default for HandlerConfig {
    fn default() -> Self {
        let test_smtp_addr = std::env::var("TEST_SMTP_ADDR")
            .ok()
            .map(|s| Url::parse(&s).expect("Failed to parse TEST_SMTP_ADDR environment variable"));

        Self { test_smtp_addr }
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

        debug!("parsing message {}", message.id());

        let json_message_data = {
            // parse and save message contents
            let message_data = MessageParser::default()
                .parse(&message.raw_data)
                .ok_or_else(|| mail_parser::Message {
                    raw_message: Borrowed(&message.raw_data),
                    ..Default::default()
                });

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

    pub async fn send_message(&self, mut message: Message) -> Result<(), HandlerError> {
        info!("sending message {}", message.id());

        for recipient in &message.recipients {
            let _domain = match email_address::EmailAddress::from_str(recipient) {
                Ok(address) => address,
                Err(err) => {
                    warn!("Invalid email address {recipient}: {err}");
                    continue;
                }
            };

            let (hostname, port) = &self
                .config
                .as_ref()
                .test_smtp_addr
                .as_ref()
                .and_then(|c| c.domain().zip(c.port()))
                .unwrap_or_else(|| {
                    warn!("No TEST_SMTP_ADDR found. Using localhost:1025; actual mail transfer is not supported yet.");
                    ("localhost", 1025)
                });

            let client = SmtpClientBuilder::new(hostname, *port);

            let mut client = match client.connect_plain().await {
                Ok(client) => {
                    trace!("connected to upstream server");

                    client
                }
                Err(e) => {
                    error!("failed to connect to upstream server: {e}");

                    self.message_repository
                        .update_message_status(&mut message, MessageStatus::Failed)
                        .await
                        .map_err(HandlerError::MessageRepositoryError)?;

                    return Err(HandlerError::ConnectToUpstream(e));
                }
            };

            // TODO FIXME: since messages can be rather large, this clone can have a negative impact on the performance
            // if the server gets under stress; and it can probably be fixed by making "update_message_status" more efficient
            // (does it really need to UPDATE the message body simply for setting the status?)
            if let Err(e) = client.send(message.clone()).await {
                error!("failed to send message: {e}");

                self.message_repository
                    .update_message_status(&mut message, MessageStatus::Failed)
                    .await
                    .map_err(HandlerError::MessageRepositoryError)?;

                return Err(HandlerError::DeliverMessage(e));
            }
        }

        self.message_repository
            .update_message_status(&mut message, MessageStatus::Delivered)
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
mod test {
    use crate::models::{SmtpCredentialRepository, SmtpCredentialRequest};
    use std::net::Ipv4Addr;

    use super::*;

    use crate::test::random_port;
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use mailcrab::TestMailServerHandle;
    use serial_test::serial;
    use tracing_test::traced_test;

    #[sqlx::test(fixtures("organizations", "domains"))]
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
            test_smtp_addr: Some(format!("smtp://localhost:{mailcrab_port}").parse().unwrap()),
        };
        let handler = Handler::new(pool, Arc::new(config), CancellationToken::new());

        let message = handler.handle_message(message).await.unwrap();
        handler.send_message(message).await.unwrap();
    }
}
