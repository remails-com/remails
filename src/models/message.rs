use chrono::{DateTime, Utc};
use mail_send::smtp::message::IntoMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type EmailAddress = String;

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type, Default)]
#[sqlx(type_name = "message_status", rename_all = "lowercase")]
pub enum MessageStatus {
    #[default]
    Processing,
    Held,
    Accepted,
    Rejected,
    Delivered,
    Failed,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    id: Uuid,
    smtp_credential_id: Uuid,
    organization_id: Uuid,
    pub status: MessageStatus,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Option<Vec<u8>>,
    pub message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub(crate) struct NewMessage {
    pub smtp_credential_id: Uuid,
    pub status: MessageStatus,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Option<Vec<u8>>,
    pub message_data: serde_json::Value,
}

impl Message {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }

    pub fn set_message_data(&mut self, message_data: serde_json::Value) {
        self.message_data = message_data;
    }

    pub fn recipients(&self) -> &[EmailAddress] {
        self.recipients.as_ref()
    }
}

impl NewMessage {
    #[cfg(test)]
    pub fn from(&self) -> &str {
        self.from_email.as_str()
    }

    pub fn add_recipient(&mut self, recipient: EmailAddress) {
        self.recipients.push(recipient);
    }

    pub fn set_raw_data(&mut self, raw_data: Vec<u8>) {
        self.raw_data = Some(raw_data);
    }

    #[cfg(test)]
    pub fn from_builder_message(
        value: mail_send::smtp::message::Message<'_>,
        smtp_credential_id: Uuid,
    ) -> Self {
        let mut message = Self {
            smtp_credential_id,
            from_email: value.mail_from.email.parse().unwrap(),
            ..Default::default()
        };
        for recipient in value.rcpt_to.iter() {
            message.recipients.push(recipient.email.parse().unwrap());
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
    pub api_user_id: Option<Uuid>,
    offset: i64,
    limit: i64,
    status: Option<MessageStatus>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            api_user_id: None,
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

    pub async fn create(&self, message: &NewMessage) -> Result<Message, sqlx::Error> {
        sqlx::query_as!(
            Message,
            r#"
            INSERT INTO messages AS m (id, smtp_credential_id, organization_id, status, from_email, recipients, raw_data, message_data)
            SELECT gen_random_uuid(), $1, o.id, $2, $3, $4, $5, $6
            FROM smtp_credential s
                JOIN domains d ON d.id = s.domain_id
                JOIN organizations o ON o.id = d.organization_id
            WHERE s.id = $1
            RETURNING m.id,
                      m.smtp_credential_id,
                      m.organization_id,
                      m.status as "status: _",
                      m.from_email,
                      m.recipients,
                      m.raw_data,
                      m.message_data,
                      m.created_at,
                      m.updated_at
            "#,
            message.smtp_credential_id,
            message.status as _,
            message.from_email,
            &message.recipients,
            message.raw_data,
            message.message_data
        )
        .fetch_one(&self.pool)
        .await
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
                m.id,
                m.smtp_credential_id,
                m.organization_id,
                m.status as "status: _",
                m.from_email,
                m.recipients,
                NULL::bytea AS "raw_data",
                NULL::jsonb AS "message_data",
                m.created_at,
                m.updated_at
            FROM messages m
                JOIN organizations o ON o.id = m.organization_id
                LEFT JOIN api_users_organizations au ON au.organization_id = o.id
                LEFT JOIN api_users u ON au.api_user_id = u.id
            WHERE ($3::message_status IS NULL OR status = $3)
              AND ($4::uuid IS NULL OR u.id = $4)
            ORDER BY created_at DESC
            OFFSET $1
            LIMIT $2
            "#,
            filter.offset,
            filter.limit,
            filter.status as _,
            filter.api_user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_by_id(
        &self,
        id: Uuid,
        api_user_id: Option<Uuid>,
    ) -> Result<Option<Message>, sqlx::Error> {
        let message = sqlx::query_as!(
            Message,
            r#"
            SELECT
                m.id,
                m.smtp_credential_id,
                m.organization_id,
                m.status as "status: _",
                m.from_email,
                m.recipients,
                m.raw_data,
                m.message_data,
                m.created_at,
                m.updated_at
            FROM messages  m
                JOIN organizations o ON o.id = m.organization_id
                LEFT JOIN api_users_organizations au ON au.organization_id = o.id
                LEFT JOIN api_users u ON au.api_user_id = u.id
            WHERE m.id = $1
              AND ($2::uuid IS NULL OR u.id = $2)
            LIMIT 1
            "#,
            id,
            api_user_id
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
    use crate::models::{SmtpCredential, SmtpCredentialRepository};

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "domains")))]
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

        let credential = SmtpCredential::new(
            "user".to_string(),
            "pass".to_string(),
            "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        );
        SmtpCredentialRepository::new(pool)
            .create(&credential)
            .await
            .unwrap();
        let new_message = NewMessage::from_builder_message(message, credential.id());

        let message = repository.create(&new_message).await.unwrap();

        let mut fetched_message = repository
            .find_by_id(message.id, None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_message.from_email, "john@example.com");

        fetched_message.recipients.sort();
        let expected = vec!["james@test.com", "jane@example.com"];

        assert_eq!(fetched_message.recipients, expected);
    }
}
