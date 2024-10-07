use anyhow::Context;
use mail_parser::MessageParser;
use mail_send::SmtpClientBuilder;
use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, info};

use crate::message::{Message, MessageRepository};

#[derive(Debug, Error)]
pub(crate) enum HandlerError {
    #[error("failed to persist message: {0}")]
    MessageRepositoryError(sqlx::Error),
    #[error("failed to parse message")]
    FailedParsingMessage,
    #[error("failed to serialize message data: {0}")]
    SerializeMessageData(serde_json::Error),
}

pub(crate) struct Handler {
    message_repository: MessageRepository,
}

impl Handler {
    pub fn new(pool: PgPool) -> Self {
        Self {
            message_repository: MessageRepository::new(pool),
        }
    }

    pub(crate) async fn handle_message(&self, message: Message) -> Result<Message, HandlerError> {
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
                .parse(message.get_raw_data())
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

        // TODO: update message status

        Ok(message)
    }

    pub(crate) async fn send_message(&self, message: Message) -> anyhow::Result<()> {
        info!("sending message {}", message.get_id());

        SmtpClientBuilder::new("localhost", 1025)
            .connect_plain()
            .await
            .context("failed connecting to SMTP server")?
            .send(message)
            .await
            .context("failed sending message")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
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

        let handler = Handler::new(pool);

        let message = handler.handle_message(message.into()).await.unwrap();
        handler.send_message(message).await.unwrap();
    }
}
