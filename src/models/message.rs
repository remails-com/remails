use crate::{
    handler::{ConnectionLog, RetryConfig},
    models::{Error, OrganizationId, SmtpCredentialId, projects::ProjectId, streams::StreamId},
};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use mail_parser::MimeHeaders;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, mem, str::FromStr};
use uuid::Uuid;

const API_RAW_TRUNCATE_LENGTH: i32 = 10_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct MessageId(Uuid);

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Serialize, sqlx::Type, Display)]
#[sqlx(type_name = "message_status", rename_all = "lowercase")]
pub enum MessageStatus {
    Processing,
    Held,
    Accepted,
    Rejected,
    Delivered,
    Reattempt,
    Failed,
}

impl MessageStatus {
    fn should_retry(&self) -> bool {
        match self {
            MessageStatus::Processing => false,
            MessageStatus::Held => true,
            MessageStatus::Accepted => false,
            MessageStatus::Rejected => false,
            MessageStatus::Delivered => false,
            MessageStatus::Reattempt => true,
            MessageStatus::Failed => false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    id: MessageId,
    pub(crate) organization_id: OrganizationId,
    pub(crate) project_id: ProjectId,
    pub(crate) stream_id: StreamId,
    pub(crate) smtp_credential_id: Option<SmtpCredentialId>,
    pub status: MessageStatus,
    pub reason: Option<String>,
    pub delivery_details: HashMap<EmailAddress, DeliveryDetails>,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    pub retry_after: Option<DateTime<Utc>>,
    pub attempts: i32,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ApiMessage {
    #[serde(flatten)]
    metadata: ApiMessageMetadata,
    pub truncated_raw_data: String,
    is_truncated: bool,
    message_data: ApiMessageData,
}

#[cfg(test)]
impl ApiMessage {
    pub fn id(&self) -> MessageId {
        self.metadata.id
    }

    pub fn smtp_credential_id(&self) -> Option<SmtpCredentialId> {
        self.metadata.smtp_credential_id
    }

    pub fn status(&self) -> &MessageStatus {
        &self.metadata.status
    }
}

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct ApiMessageMetadata {
    pub id: MessageId,
    pub status: MessageStatus,
    reason: Option<String>,
    delivery_details: HashMap<EmailAddress, DeliveryDetails>,
    smtp_credential_id: Option<SmtpCredentialId>,
    pub from_email: EmailAddress,
    recipients: Vec<EmailAddress>,
    /// Human-readable size
    raw_size: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    retry_after: Option<DateTime<Utc>>,
    attempts: i32,
    max_attempts: i32,
}

#[derive(Serialize, Default)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ApiMessageData {
    pub subject: Option<String>,
    /// An RFC3339 String
    pub date: Option<String>,
    pub text_body: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct Attachment {
    pub filename: String,
    pub mime: String,
    /// Human-readable size
    pub size: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum DeliveryStatus {
    Success { delivered: DateTime<Utc> },
    Reattempt,
    Failed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeliveryDetails {
    pub status: DeliveryStatus,
    pub log: ConnectionLog,
}

impl DeliveryDetails {
    pub fn new(status: DeliveryStatus, log: ConnectionLog) -> Self {
        Self { status, log }
    }
}

#[derive(Debug)]
pub struct NewMessage {
    pub smtp_credential_id: SmtpCredentialId,
    pub status: MessageStatus,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(Deserialize))]
pub struct MessageRetryUpdate {
    status: MessageStatus,
    retry_after: Option<DateTime<Utc>>,
    attempts: i32,
    max_attempts: i32,
}

impl Message {
    pub fn id(&self) -> MessageId {
        self.id
    }

    pub fn prepend_headers(&mut self, headers: &str) {
        // TODO: we could 'overallocate' the original raw message data to prepend this stuff without
        // needing to allocate or move data around.
        let hdr_size = headers.len();
        let msg_len = self.raw_data.len();

        self.raw_data.resize(msg_len + hdr_size, Default::default());
        self.raw_data.copy_within(..msg_len, hdr_size);
        self.raw_data[..hdr_size].copy_from_slice(headers.as_bytes());
    }

    pub fn set_next_retry(&mut self, config: &RetryConfig) {
        self.attempts += 1;

        if !self.status.should_retry() {
            self.retry_after = None;
            return;
        }

        if self.attempts < config.max_automatic_retries {
            let timeout = config
                .delay
                .checked_mul(self.attempts)
                .unwrap_or(chrono::TimeDelta::days(1));
            self.retry_after = Some(chrono::Utc::now() + timeout);
        } else {
            match &self.status {
                MessageStatus::Held => self.status = MessageStatus::Rejected,
                MessageStatus::Reattempt => self.status = MessageStatus::Failed,
                _ => {}
            };
            self.retry_after = None;
        }
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
}

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct MessageFilter {
    limit: i64,
    status: Option<MessageStatus>,
    before: Option<DateTime<Utc>>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            limit: 10, // should match LIMIT_DEFAULT in frontend/src/components/messages/MessageLog.tsx
            status: None,
            before: None,
        }
    }
}

struct PgMessage {
    id: MessageId,
    organization_id: OrganizationId,
    project_id: ProjectId,
    stream_id: StreamId,
    smtp_credential_id: Option<Uuid>,
    status: MessageStatus,
    reason: Option<String>,
    delivery_details: serde_json::Value,
    from_email: String,
    recipients: Vec<String>,
    raw_data: Vec<u8>,
    raw_size: i32,
    message_data: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    retry_after: Option<DateTime<Utc>>,
    attempts: i32,
    max_attempts: i32,
}

impl TryFrom<PgMessage> for Message {
    type Error = super::Error;

    fn try_from(m: PgMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            id: m.id,
            organization_id: m.organization_id,
            project_id: m.project_id,
            stream_id: m.stream_id,
            smtp_credential_id: m.smtp_credential_id.map(Into::into),
            status: m.status,
            reason: m.reason,
            delivery_details: serde_json::from_value(m.delivery_details)?,
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
            retry_after: m.retry_after,
            attempts: m.attempts,
        })
    }
}

impl From<mail_parser::Message<'_>> for ApiMessageData {
    fn from(m: mail_parser::Message<'_>) -> Self {
        // TODO get rid of as many allocations as possible here
        Self {
            subject: m.subject().map(|s| s.to_string()),
            text_body: m
                .text_bodies()
                .next()
                .and_then(|m| m.text_contents())
                .map(|t| t.to_string()),
            date: m.date().map(mail_parser::DateTime::to_rfc3339),
            attachments: m.attachments().map(Into::into).collect(),
        }
    }
}

impl From<&mail_parser::MessagePart<'_>> for Attachment {
    fn from(part: &mail_parser::MessagePart) -> Self {
        let filename = part.attachment_name().unwrap_or_default().to_string();
        let mime = match part.content_type() {
            Some(content_type) => match &content_type.c_subtype {
                Some(subtype) => format!("{}/{}", content_type.c_type, subtype),
                None => content_type.c_type.to_string(),
            },
            None => "application/octet-stream".to_owned(),
        };

        Attachment {
            filename,
            mime,
            size: humansize::format_size(part.contents().len(), humansize::DECIMAL),
        }
    }
}

impl TryFrom<PgMessage> for ApiMessage {
    type Error = super::Error;

    fn try_from(mut m: PgMessage) -> Result<Self, Self::Error> {
        let message_data_json = mem::take(&mut m.message_data);
        let message_data: Option<mail_parser::Message> = serde_json::from_value(message_data_json)?;
        let raw_data_bytes = mem::take(&mut m.raw_data);
        Ok(Self {
            truncated_raw_data: String::from_utf8(raw_data_bytes)?,
            is_truncated: m.raw_size > API_RAW_TRUNCATE_LENGTH,
            message_data: message_data.map(Into::into).unwrap_or_default(),
            metadata: m.try_into()?,
        })
    }
}

impl TryFrom<PgMessage> for ApiMessageMetadata {
    type Error = super::Error;

    fn try_from(m: PgMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            id: m.id,
            status: m.status,
            reason: m.reason,
            delivery_details: serde_json::from_value(m.delivery_details)?,
            smtp_credential_id: m.smtp_credential_id.map(Into::into),
            from_email: EmailAddress::from_str(&m.from_email)?,
            recipients: m
                .recipients
                .iter()
                .map(|addr| addr.parse())
                .collect::<Result<Vec<_>, _>>()?,
            raw_size: humansize::format_size(m.raw_size.unsigned_abs(), humansize::DECIMAL),
            created_at: m.created_at,
            updated_at: m.updated_at,
            retry_after: m.retry_after,
            attempts: m.attempts,
            max_attempts: m.max_attempts,
        })
    }
}

impl MessageRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, message: &NewMessage, max_attempts: i32) -> Result<Message, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            INSERT INTO messages AS m (id, organization_id, project_id, stream_id, smtp_credential_id, status, from_email, recipients, raw_data, message_data, max_attempts)
            SELECT gen_random_uuid(), o.id, p.id, streams.id, $1, $2, $3, $4, $5, $6, $7
            FROM smtp_credentials s
                JOIN streams ON s.stream_id = streams.id
                JOIN projects p ON p.id = streams.project_id
                JOIN organizations o ON o.id = p.organization_id
            WHERE s.id = $1
            RETURNING
                m.id,
                m.organization_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.reason,
                m.delivery_details,
                m.from_email,
                m.recipients,
                m.raw_data,
                octet_length(m.raw_data) as "raw_size!",
                m.message_data,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            "#,
            *message.smtp_credential_id,
            message.status as _,
            message.from_email.as_str(),
            &message.recipients.iter().map(|r| r.email()).collect::<Vec<_>>(),
            message.raw_data,
            message.message_data,
            max_attempts
        )
            .fetch_one(&self.pool)
            .await?
            .try_into()
    }

    pub async fn update_message_status(&self, message: &mut Message) -> Result<(), Error> {
        let delivery_details_serialized =
            serde_json::to_value(&message.delivery_details).map_err(Error::Serialization)?;

        sqlx::query!(
            r#"
            UPDATE messages
            SET status = $2,
                reason = $3,
                delivery_details = $4,
                retry_after = $5,
                attempts = $6
            WHERE id = $1
            "#,
            *message.id,
            message.status as _,
            message.reason,
            delivery_details_serialized,
            message.retry_after,
            message.attempts,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_message_data_and_status(&self, message: &Message) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET message_data = $2,
                status = $3,
                reason = $4,
                retry_after = $5,
                attempts = $6
            WHERE id = $1
            "#,
            *message.id,
            message.message_data,
            message.status as _,
            message.reason,
            message.retry_after,
            message.attempts,
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
    ) -> Result<Vec<ApiMessageMetadata>, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                id,
                organization_id,
                project_id,
                stream_id,
                smtp_credential_id,
                status AS "status: _",
                reason,
                delivery_details,
                from_email,
                recipients,
                ''::bytea AS "raw_data!",
                NULL::jsonb AS "message_data",
                octet_length(raw_data) AS "raw_size!",
                created_at,
                updated_at,
                retry_after,
                attempts,
                max_attempts
            FROM messages m
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR project_id = $2)
                AND ($3::uuid IS NULL OR stream_id = $3)
                AND ($5::message_status IS NULL OR status = $5)
                AND ($6::timestamptz IS NULL OR created_at <= $6)
            ORDER BY created_at DESC
            LIMIT $4
            "#,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
            std::cmp::min(filter.limit, 100) + 1, // plus one to indicate there are more entries available
            filter.status as _,
            filter.before,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, Error>>()
    }

    /// Get a specific message
    ///
    /// Unlike [`find_by_id`] this returns a `Message` with the full raw data
    pub async fn get(&self, message_id: MessageId) -> Result<Message, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.reason,
                m.delivery_details,
                m.from_email,
                m.recipients,
                m.raw_data,
                octet_length(m.raw_data) as "raw_size!",
                m.message_data,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            FROM messages m
            WHERE m.id = $1
            "#,
            *message_id,
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }

    pub async fn find_by_id(
        &self,
        org_id: OrganizationId,
        project_id: Option<ProjectId>,
        stream_id: Option<StreamId>,
        message_id: MessageId,
    ) -> Result<ApiMessage, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.status as "status: _",
                m.reason,
                m.delivery_details,
                m.from_email,
                m.recipients,
                -- Only return the first API_RAW_TRUNCATE_LENGTH bytes/ASCII-characters of the raw data.
                substring(m.raw_data FOR $5) as "raw_data!",
                octet_length(m.raw_data) as "raw_size!",
                m.message_data,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            FROM messages m
            WHERE m.id = $1
              AND m.organization_id = $2
              AND ($3::uuid IS NULL OR m.project_id = $3)
              AND ($4::uuid IS NULL OR m.stream_id = $4)
            "#,
            *message_id,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
            API_RAW_TRUNCATE_LENGTH,
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }

    pub async fn remove(
        &self,
        org_id: OrganizationId,
        project_id: Option<ProjectId>,
        stream_id: Option<StreamId>,
        message_id: MessageId,
    ) -> Result<MessageId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM messages
            WHERE id = $1
              AND organization_id = $2
              AND ($3::uuid IS NULL OR project_id = $3)
              AND ($4::uuid IS NULL OR stream_id = $4)
            RETURNING id
            "#,
            *message_id,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    pub async fn find_messages_ready_for_retry(&self) -> Result<Vec<MessageId>, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT m.id
            FROM messages m
            WHERE (m.status = 'held' OR m.status = 'reattempt')
              AND now() > m.retry_after
              AND m.attempts < m.max_attempts
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    pub async fn update_to_retry_asap(
        &self,
        org_id: OrganizationId,
        project_id: Option<ProjectId>,
        stream_id: Option<StreamId>,
        message_id: MessageId,
    ) -> Result<MessageRetryUpdate, Error> {
        Ok(sqlx::query_as!(
            MessageRetryUpdate,
            r#"
            UPDATE messages
            SET retry_after = now(),
                max_attempts = GREATEST(attempts + 1, max_attempts),
                status = CASE status
                    WHEN 'rejected' THEN 'held'
                    WHEN 'failed' THEN 'reattempt'
                    ELSE status
                END
            WHERE id = $1
              AND organization_id = $2
              AND ($3::uuid IS NULL OR project_id = $3)
              AND ($4::uuid IS NULL OR stream_id = $4)
            RETURNING status as "status: _", retry_after, attempts, max_attempts
            "#,
            *message_id,
            *org_id,
            project_id.map(|p| p.as_uuid()),
            stream_id.map(|s| s.as_uuid()),
        )
        .fetch_one(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod test {
    use mail_send::{mail_builder::MessageBuilder, smtp::message::IntoMessage};
    use sqlx::PgPool;

    use super::*;
    use crate::{
        models::{SmtpCredentialRepository, SmtpCredentialRequest},
        test::TestStreams,
    };

    impl NewMessage {
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

        pub fn from_builder_message_custom_from(
            value: mail_send::smtp::message::Message<'_>,
            smtp_credential_id: SmtpCredentialId,
            smtp_from: &str,
        ) -> Self {
            use mail_send::smtp::message::IntoMessage;
            let mut message = Self::new(smtp_credential_id, smtp_from.parse().unwrap());
            for recipient in value.rcpt_to.iter() {
                message.recipients.push(recipient.email.parse().unwrap());
            }
            message.raw_data = value.into_message().unwrap().body.to_vec();

            message
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains", "streams")
    ))]
    async fn message_repository(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());

        let message = MessageBuilder::new()
            .from(("John Doe", "john@test-org-1-project-1.com"))
            .to(vec![
                ("James Smith", "james@test.com"),
                ("Jane Doe", "jane@test-org-1-project-1.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .into_message()
            .unwrap();
        let smtp_credential_repo = SmtpCredentialRepository::new(pool);

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

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

        let message = repository.create(&new_message, 5).await.unwrap();

        let mut fetched_message = repository
            .find_by_id(org_id, Some(project_id), Some(stream_id), message.id)
            .await
            .unwrap();

        assert_eq!(
            fetched_message.metadata.from_email,
            "john@test-org-1-project-1.com".parse().unwrap()
        );

        fetched_message
            .metadata
            .recipients
            .sort_by_key(|x| x.email());
        let expected = vec![
            "james@test.com".parse().unwrap(),
            "jane@test-org-1-project-1.com".parse().unwrap(),
        ];

        assert_eq!(fetched_message.metadata.recipients, expected);

        let messages = repository
            .list_message_metadata(
                org_id,
                Some(project_id),
                Some(stream_id),
                MessageFilter {
                    limit: 5,
                    status: None,
                    before: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, message.id);

        repository
            .remove(org_id, Some(project_id), Some(stream_id), message.id)
            .await
            .unwrap();

        let messages = repository
            .list_message_metadata(
                org_id,
                Some(project_id),
                Some(stream_id),
                MessageFilter {
                    limit: 5,
                    status: None,
                    before: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(messages.len(), 0);
    }
}
