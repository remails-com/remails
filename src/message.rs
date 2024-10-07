use mail_send::smtp::message::IntoMessage;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

pub(crate) type EmailAddress = String;

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct Message {
    id: Uuid,
    from_email: EmailAddress,
    recipients: Vec<EmailAddress>,
    raw_data: Vec<u8>,
    message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Message {
    pub fn new(from_email: EmailAddress) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            from_email,
            recipients: Vec::new(),
            raw_data: Vec::new(),
            message_data: serde_json::Value::Null,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    #[cfg(test)]
    pub fn get_from(&self) -> &str {
        &self.from_email
    }

    pub fn get_raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    pub fn add_recipient(&mut self, recipient: EmailAddress) {
        self.recipients.push(recipient);
    }

    pub fn set_raw_data(&mut self, raw_data: Vec<u8>) {
        self.raw_data = raw_data;
    }

    pub fn set_message_data(&mut self, message_data: serde_json::Value) {
        self.message_data = message_data;
    }
}

impl<'x> IntoMessage<'x> for Message {
    fn into_message(self) -> mail_send::Result<mail_send::smtp::message::Message<'x>> {
        Ok(mail_send::smtp::message::Message {
            mail_from: self.from_email.into(),
            rcpt_to: self.recipients.into_iter().map(|m| m.into()).collect(),
            body: self.raw_data.into(),
        })
    }
}

impl From<mail_send::smtp::message::Message<'_>> for Message {
    fn from(value: mail_send::smtp::message::Message<'_>) -> Self {
        let mut message = Message::new(value.mail_from.email.clone().into());
        for recipient in value.rcpt_to.iter() {
            message.add_recipient(recipient.email.clone().into());
        }
        message.raw_data = value.into_message().unwrap().body.to_vec();

        message
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MessageRepository {
    pool: sqlx::PgPool,
}

impl MessageRepository {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn insert(&self, message: &Message) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO messages (id, from_email, recipients, raw_data, message_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            message.id,
            message.from_email,
            &message.recipients,
            message.raw_data,
            message.message_data,
            message.created_at,
            message.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn update_message_data(&self, message: &Message) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET message_data = $2
            WHERE id = $1
            "#,
            message.id,
            message.message_data,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn find_by_id(&self, id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        let message = sqlx::query_as!(
            Message,
            r#"
            SELECT * FROM messages WHERE id = $1 LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(message)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use mail_send::mail_builder::MessageBuilder;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn message_repository(pool: PgPool) {
        let repository = MessageRepository::new(pool);

        let message: Message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("Jane Doe", "jane@example.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .into_message()
            .unwrap()
            .into();

        let id = message.id;

        repository.insert(&message).await.unwrap();

        let fetched_message = repository.find_by_id(id).await.unwrap().unwrap();

        assert_eq!(fetched_message.from_email, "john@example.com");
        assert_eq!(
            fetched_message.recipients,
            vec!["jane@example.com", "james@test.com"]
        );
    }
}
