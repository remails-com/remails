use anyhow::Context;
use mail_parser::MessageParser;
use mail_send::SmtpClientBuilder;
use sqlx::PgPool;
use tracing::{debug, info};

use crate::message::{Message, MessageRepository};

pub(crate) async fn handle_message(message: Message, pool: PgPool) -> anyhow::Result<Message> {
    debug!("storing message {}", message.get_id());

    let repo = MessageRepository::new(pool);
    repo.insert(&message).await?;

    // TODO: check limits etc

    debug!("parsing message {}", message.get_id());

    let json_message_data = {
        // parse and save message contents
        let message_data = MessageParser::default()
            .parse(message.get_raw_data())
            .context(format!("Failed parsing message {}", message.get_id()))?;

        serde_json::to_value(&message_data).context("failed converting message data to JSON")?
    };

    debug!("updating message {}", message.get_id());

    let mut message = message;
    message.set_message_data(json_message_data);
    repo.update_message_data(&message).await?;

    // TODO: update message status

    Ok(message)
}

pub(crate) async fn send_message(message: Message) -> anyhow::Result<()> {
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

        let message = handle_message(message.into(), pool).await.unwrap();
        send_message(message).await.unwrap();
    }
}
