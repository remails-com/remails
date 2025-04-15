use crate::models::{
    Error, OrganizationId, SmtpCredentialId, domains::DomainId, projects::ProjectId,
    streams::StreamId,
};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type EmailAddress = String;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref)]
pub struct MessageId(Uuid);

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
    id: MessageId,
    organization_id: OrganizationId,
    domain_id: Option<DomainId>,
    project_id: ProjectId,
    stream_id: StreamId,
    smtp_credential_id: Option<SmtpCredentialId>,
    pub status: MessageStatus,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub(crate) struct NewMessage {
    pub smtp_credential_id: SmtpCredentialId,
    pub status: MessageStatus,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
}

impl Message {
    pub fn id(&self) -> MessageId {
        self.id
    }
}

impl NewMessage {
    pub fn new(smtp_credential_id: SmtpCredentialId, from_email: EmailAddress) -> Self {
        NewMessage {
            smtp_credential_id,
            status: MessageStatus::Processing,
            from_email,
            recipients: vec![],
            raw_data: vec![],
            message_data: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn from_builder_message(
        value: mail_send::smtp::message::Message<'_>,
        smtp_credential_id: SmtpCredentialId,
    ) -> Self {
        use mail_send::smtp::message::IntoMessage;
        let mut message = Self::new(smtp_credential_id, value.mail_from.email.parse().unwrap());
        for recipient in value.rcpt_to.iter() {
            message.recipients.push(recipient.email.parse().unwrap());
        }
        message.raw_data = value.into_message().unwrap().body.to_vec();

        message
    }
}

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct MessageFilter {
    pub orgs: Option<Vec<OrganizationId>>,
    offset: i64,
    limit: i64,
    status: Option<MessageStatus>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            orgs: None,
            offset: 0,
            limit: 100,
            status: None,
        }
    }
}

impl MessageFilter {
    fn org_uuids(&self) -> Option<Vec<Uuid>> {
        self.orgs
            .as_deref()
            .map(|o| o.iter().map(|o| o.as_uuid()).collect())
    }
}

struct PgMessage {
    id: MessageId,
    organization_id: OrganizationId,
    domain_id: Option<Uuid>,
    project_id: ProjectId,
    stream_id: StreamId,
    smtp_credential_id: Option<Uuid>,
    status: MessageStatus,
    from_email: EmailAddress,
    recipients: Vec<EmailAddress>,
    raw_data: Vec<u8>,
    message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PgMessage> for Message {
    fn from(m: PgMessage) -> Self {
        Self {
            id: m.id,
            organization_id: m.organization_id,
            domain_id: m.domain_id.map(Into::into),
            project_id: m.project_id,
            stream_id: m.stream_id,
            smtp_credential_id: m.smtp_credential_id.map(Into::into),
            status: m.status,
            from_email: m.from_email,
            recipients: m.recipients,
            raw_data: m.raw_data,
            message_data: m.message_data,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

impl MessageRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, message: &NewMessage) -> Result<Message, Error> {
        Ok(sqlx::query_as!(
            PgMessage,
            r#"
            INSERT INTO messages AS m (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, from_email, recipients, raw_data, message_data)
            SELECT gen_random_uuid(), o.id, COALESCE(d_p.id, d_o.id), p.id, streams.id, $1, $2, $3, $4, $5, $6
            FROM smtp_credentials s
                JOIN streams ON s.stream_id = streams.id
                JOIN projects p ON p.id = streams.project_id
                JOIN organizations o ON o.id = p.organization_id
                LEFT JOIN domains d_p ON d_p.project_id = p.id
                LEFT JOIN domains d_o ON d_o.organization_id = o.id
            WHERE s.id = $1
            RETURNING
                m.id,
                m.organization_id,
                m.domain_id AS "domain_id: Uuid",
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.from_email,
                m.recipients,
                m.raw_data,
                m.message_data,
                m.created_at,
                m.updated_at
            "#,
            *message.smtp_credential_id,
            message.status as _,
            message.from_email,
            &message.recipients,
            message.raw_data,
            message.message_data
        )
            .fetch_one(&self.pool)
            .await?
            .into())
    }

    pub async fn update_message_status(
        &self,
        message: &mut Message,
        status: MessageStatus,
    ) -> Result<(), Error> {
        message.status = status;

        sqlx::query!(
            r#"
            UPDATE messages
            SET status = $2
            WHERE id = $1
            "#,
            *message.id,
            message.status as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_message_data(&self, message: &Message) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET message_data = $2
            WHERE id = $1
            "#,
            *message.id,
            message.message_data,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_message_metadata(
        &self,
        filter: MessageFilter,
    ) -> Result<Vec<Message>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.domain_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.from_email,
                m.recipients,
                ''::bytea AS "raw_data!",
                NULL::jsonb AS "message_data",
                m.created_at,
                m.updated_at
            FROM messages m
                JOIN organizations o ON o.id = m.organization_id
            WHERE ($3::message_status IS NULL OR status = $3)
              AND ($4::uuid[] IS NULL OR o.id = ANY($4))
            ORDER BY created_at DESC
            OFFSET $1
            LIMIT $2
            "#,
            filter.offset,
            filter.limit,
            filter.status as _,
            orgs.as_deref(),
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    pub async fn find_by_id(
        &self,
        id: MessageId,
        filter: MessageFilter,
    ) -> Result<Option<Message>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.domain_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.from_email,
                m.recipients,
                m.raw_data,
                m.message_data,
                m.created_at,
                m.updated_at
            FROM messages  m
                JOIN organizations o ON o.id = m.organization_id
            WHERE m.id = $1
              AND ($2::uuid[] IS NULL OR o.id = ANY($2))
            LIMIT 1
            "#,
            *id,
            orgs.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Into::into))
    }
}

#[cfg(test)]
mod test {
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use sqlx::PgPool;

    use super::*;
    use crate::models::{SmtpCredentialRepository, SmtpCredentialRequest};

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects" , "domains", "streams")
    ))]
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
        let smtp_credential_repo = SmtpCredentialRepository::new(pool);
        let credential = smtp_credential_repo
            .generate(&SmtpCredentialRequest {
                username: "user".to_string(),
                stream_id: "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
                description: "Test SMTP credential description".to_string(),
            })
            .await
            .unwrap();

        let new_message = NewMessage::from_builder_message(message, credential.id());

        let message = repository.create(&new_message).await.unwrap();

        let mut fetched_message = repository
            .find_by_id(message.id, MessageFilter::default())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched_message.from_email, "john@example.com");

        fetched_message.recipients.sort();
        let expected = vec!["james@test.com", "jane@example.com"];

        assert_eq!(fetched_message.recipients, expected);
    }
}
