use chrono::{DateTime, Utc};
use mail_send::smtp::message::IntoMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type EmailAddress = String;

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "message_status", rename_all = "lowercase")]
pub enum MessageStatus {
    Processing,
    Held,
    Accepted,
    Rejected,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    id: Uuid,
    smtp_credential_id: Uuid,
    status: MessageStatus,
    from_email: EmailAddress,
    recipients: Vec<EmailAddress>,
    raw_data: Option<Vec<u8>>,
    message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Message {
    pub fn new(smtp_credential_id: Uuid, from_email: EmailAddress) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            smtp_credential_id,
            status: MessageStatus::Processing,
            from_email,
            recipients: Vec::new(),
            raw_data: None,
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
        self.from_email.as_str()
    }

    pub fn get_raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }

    pub fn add_recipient(&mut self, recipient: EmailAddress) {
        self.recipients.push(recipient);
    }

    pub fn get_recipients(&self) -> &[EmailAddress] {
        self.recipients.as_ref()
    }

    pub fn set_raw_data(&mut self, raw_data: Vec<u8>) {
        self.raw_data = Some(raw_data);
    }

    pub fn set_message_data(&mut self, message_data: serde_json::Value) {
        self.message_data = message_data;
    }

    #[cfg(test)]
    pub fn from_builder_message(
        value: mail_send::smtp::message::Message<'_>,
        user_id: Uuid,
    ) -> Self {
        let mut message = Message::new(user_id, value.mail_from.email.parse().unwrap());
        for recipient in value.rcpt_to.iter() {
            message.add_recipient(recipient.email.parse().unwrap());
        }
        message.raw_data = Some(value.into_message().unwrap().body.to_vec());

        message
    }
}

impl<'x> IntoMessage<'x> for Message {
    fn into_message(self) -> mail_send::Result<mail_send::smtp::message::Message<'x>> {
        Ok(mail_send::smtp::message::Message {
            mail_from: self.from_email.into(),
            rcpt_to: self.recipients.into_iter().map(|m| m.into()).collect(),
            body: self.raw_data.unwrap_or_default().into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct MessageFilter {
    pub user_id: Option<Uuid>,
    offset: i64,
    limit: i64,
    status: Option<MessageStatus>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            user_id: None,
            offset: 0,
            limit: 100,
            status: None,
        }
    }
}

impl MessageRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, message: &Message) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO messages (id, smtp_credential_id, status, from_email, recipients, raw_data, message_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            message.id,
            message.smtp_credential_id,
            message.status as _,
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

    pub async fn update_message_status(
        &self,
        message: &mut Message,
        status: MessageStatus,
    ) -> Result<(), sqlx::Error> {
        message.status = status;

        sqlx::query!(
            r#"
            UPDATE messages
            SET status = $2
            WHERE id = $1
            "#,
            message.id,
            message.status as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_message_data(&self, message: &Message) -> Result<(), sqlx::Error> {
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

    pub async fn list_message_metadata(
        &self,
        filter: MessageFilter,
    ) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as!(
            Message,
            r#"
            SELECT
                id,
                smtp_credential_id,
                status as "status: _",
                from_email,
                recipients,
                NULL::bytea AS "raw_data",
                NULL::jsonb AS "message_data",
                created_at,
                updated_at
            FROM messages
            WHERE ($3::message_status IS NULL OR status = $3)
            AND ($4::uuid IS NULL OR smtp_credential_id = $4)
            ORDER BY created_at DESC
            OFFSET $1
            LIMIT $2
            "#,
            filter.offset,
            filter.limit,
            filter.status as _,
            filter.user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        let message = sqlx::query_as!(
            Message,
            r#"
            SELECT
                id,
                smtp_credential_id,
                status as "status: _",
                from_email,
                recipients,
                raw_data,
                message_data,
                created_at,
                updated_at
            FROM messages WHERE id = $1 LIMIT 1
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
    use mail_send::mail_builder::MessageBuilder;
    use sqlx::PgPool;

    use super::*;
    use crate::smtp_credential::{SmtmCredential, SmtpCredentialRepository};

    #[sqlx::test]
    async fn message_repository(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());

        let message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("James Smith", "james@test.com"),
                ("Jane Doe", "jane@example.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .into_message()
            .unwrap();

        let credential = SmtmCredential::new("user".to_string(), "pass".to_string());
        SmtpCredentialRepository::new(pool)
            .insert(&credential)
            .await
            .unwrap();
        let message = Message::from_builder_message(message, credential.get_id());

        let id = message.id;

        repository.insert(&message).await.unwrap();

        let mut fetched_message = repository.find_by_id(id).await.unwrap().unwrap();

        assert_eq!(fetched_message.from_email, "john@example.com");

        fetched_message.recipients.sort();
        let expected = vec!["james@test.com", "jane@example.com"];

        assert_eq!(fetched_message.recipients, expected);
    }
}
