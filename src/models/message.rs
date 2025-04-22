use crate::models::{
    Error, OrganizationId, SmtpCredentialId, domains::DomainId, projects::ProjectId,
    streams::StreamId,
};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
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
    offset: i64,
    limit: i64,
    status: Option<MessageStatus>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 100,
            status: None,
        }
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
    from_email: String,
    recipients: Vec<String>,
    raw_data: Vec<u8>,
    message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgMessage> for Message {
    type Error = super::Error;

    fn try_from(m: PgMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            id: m.id,
            organization_id: m.organization_id,
            domain_id: m.domain_id.map(Into::into),
            project_id: m.project_id,
            stream_id: m.stream_id,
            smtp_credential_id: m.smtp_credential_id.map(Into::into),
            status: m.status,
            from_email: EmailAddress::from_str(&m.from_email)?,
            recipients: m
                .recipients
                .iter()
                .map(|addr| addr.parse())
                .collect::<Result<Vec<_>, _>>()?,
            raw_data: m.raw_data,
            message_data: m.message_data,
            created_at: m.created_at,
            updated_at: m.updated_at,
        })
    }
}

impl MessageRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, message: &NewMessage) -> Result<Message, Error> {
        sqlx::query_as!(
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
            message.from_email.as_str(),
            &message.recipients.iter().map(|r| r.email()).collect::<Vec<_>>(),
            message.raw_data,
            message.message_data
        )
            .fetch_one(&self.pool)
            .await?
            .try_into()
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
        org_id: OrganizationId,
        project_id: Option<ProjectId>,
        stream_id: Option<StreamId>,
        filter: MessageFilter,
    ) -> Result<Vec<Message>, Error> {
        sqlx::query_as!(
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
            WHERE ($3::message_status IS NULL OR status = $3)
              AND m.organization_id = $4 
              AND ($5::uuid IS NULL OR m.project_id = $5) 
              AND ($6::uuid IS NULL OR m.stream_id = $6)
            ORDER BY created_at DESC
            OFFSET $1
            LIMIT $2
            "#,
            filter.offset,
            filter.limit,
            filter.status as _,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, Error>>()
    }

    pub async fn find_by_id(
        &self,
        org_id: OrganizationId,
        project_id: Option<ProjectId>,
        stream_id: Option<StreamId>,
        message_id: MessageId,
    ) -> Result<Message, Error> {
        sqlx::query_as!(
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
            WHERE m.id = $1
              AND m.organization_id = $2 
              AND ($3::uuid IS NULL OR m.project_id = $3) 
              AND ($4::uuid IS NULL OR m.stream_id = $4)
            "#,
            *message_id,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }
}

#[cfg(test)]
mod test {
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use sqlx::PgPool;

    use super::*;
    use crate::models::{SmtpCredentialRepository, SmtpCredentialRequest};

    impl Message {
        pub fn smtp_credential_id(&self) -> Option<SmtpCredentialId> {
            self.smtp_credential_id
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
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

        let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

        let credential = smtp_credential_repo
            .generate(
                org_id,
                project_id,
                stream_id,
                &SmtpCredentialRequest {
                    username: "user".to_string(),
                    description: "Test SMTP credential description".to_string(),
                },
            )
            .await
            .unwrap();

        let new_message = NewMessage::from_builder_message(message, credential.id());

        let message = repository.create(&new_message).await.unwrap();

        let mut fetched_message = repository
            .find_by_id(
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                Some("85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap()),
                message.id,
            )
            .await
            .unwrap();

        assert_eq!(
            fetched_message.from_email,
            "john@example.com".parse().unwrap()
        );

        fetched_message.recipients.sort_by_key(|x| x.email());
        let expected = vec![
            "james@test.com".parse().unwrap(),
            "jane@example.com".parse().unwrap(),
        ];

        assert_eq!(fetched_message.recipients, expected);
    }
}
