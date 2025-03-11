use std::str::FromStr;
use mail_parser::MessageParser;
use mail_send::SmtpClientBuilder;
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use tokio_rustls::rustls::{crypto, crypto::CryptoProvider};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

use crate::message::{Message, MessageRepository, MessageStatus};

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("failed to persist message: {0}")]
    MessageRepositoryError(sqlx::Error),
    #[error("failed to parse message")]
    FailedParsingMessage,
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
    #[error("failed to connect to upstream server: {0}")]
    ConnectToUpstream(mail_send::Error),
    #[error("failed to deliver message: {0}")]
    DeliverMessage(mail_send::Error),
}

pub struct Handler {
    message_repository: MessageRepository,
    shutdown: CancellationToken,
}

impl Handler {
    pub fn new(pool: PgPool, shutdown: CancellationToken) -> Self {
        if CryptoProvider::get_default().is_none() {
            CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
                .expect("Failed to install crypto provider");
        }
        Self {
            message_repository: MessageRepository::new(pool),
            shutdown,
        }
    }

    pub async fn handle_message(&self, message: Message) -> Result<Message, HandlerError> {
        debug!("storing message {}", message.get_id());

        self.message_repository
            .insert(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        // TODO: check limits etc

        debug!("parsing message {}", message.get_id());

        let json_message_data = {
            // parse and save message contents
            let message_data = MessageParser::default()
                .parse(message.get_raw_data().unwrap_or_default())
                .ok_or(HandlerError::FailedParsingMessage)?;

            serde_json::to_value(&message_data).map_err(HandlerError::SerializeMessageData)?
        };

        debug!("updating message {}", message.get_id());

        let mut message = message;
        message.set_message_data(json_message_data);

        self.message_repository
            .update_message_data(&message)
            .await
            .map_err(HandlerError::MessageRepositoryError)?;

        Ok(message)
    }

    pub async fn send_message(&self, mut message: Message) -> Result<(), HandlerError> {
        info!("sending message {}", message.get_id());

        for recipient in message.get_recipients() {
            let domain = match email_address::EmailAddress::from_str(recipient) {
                Ok(address) => address,
                Err(err) => {
                    warn!("Invalid email address {recipient}: {err}");
                    continue;
                }
            };

            let client = SmtpClientBuilder::new(domain, 587);
            #[cfg(test)]
            let client = SmtpClientBuilder::new("localhost", 1025).allow_invalid_certs();

            let mut client = match client.connect().await {
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

    pub fn spawn(self, mut queue_receiver: Receiver<Message>) {
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
    use crate::user::{User, UserRepository};

    use super::*;

    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use tracing_test::traced_test;

    #[sqlx::test]
    #[traced_test]
    async fn test_handle_message(pool: PgPool) {
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

        let user = User::new("user".to_string(), "pass".to_string());
        UserRepository::new(pool.clone())
            .insert(&user)
            .await
            .unwrap();

        let message = Message::from_builder_message(message, user.get_id());
        let handler = Handler::new(pool, CancellationToken::new());

        let message = handler.handle_message(message).await.unwrap();
        handler.send_message(message).await.unwrap();
    }
}
