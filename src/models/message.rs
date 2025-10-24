use crate::{
    handler::{ConnectionLog, RetryConfig},
    models::{
        ApiKeyId, Error, OrganizationId, SmtpCredentialId, projects::ProjectId, streams::StreamId,
    },
};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use mail_parser::MimeHeaders;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::types::PgInterval, types::ipnet::IpNet};
use std::{collections::HashMap, mem, net::IpAddr, str::FromStr};
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
    pub(crate) api_key_id: Option<ApiKeyId>,
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
    pub max_attempts: i32,
    pub outbound_ip: Option<IpAddr>,
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

    pub fn api_key_id(&self) -> Option<ApiKeyId> {
        self.metadata.api_key_id
    }

    pub fn status(&self) -> &MessageStatus {
        &self.metadata.status
    }

    pub fn outbound_ip(&self) -> Option<IpAddr> {
        self.metadata.outbound_ip
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
    api_key_id: Option<ApiKeyId>,
    pub from_email: EmailAddress,
    recipients: Vec<EmailAddress>,
    /// Human-readable size
    raw_size: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    retry_after: Option<DateTime<Utc>>,
    attempts: i32,
    max_attempts: i32,
    outbound_ip: Option<IpAddr>,
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

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(tag = "type")]
pub enum DeliveryStatus {
    #[default]
    None,
    Success {
        delivered: DateTime<Utc>,
    },
    Reattempt,
    Failed,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct DeliveryDetails {
    pub status: DeliveryStatus,
    pub log: ConnectionLog,
}

impl DeliveryDetails {
    pub fn new(status: DeliveryStatus, log: ConnectionLog) -> Self {
        Self { status, log }
    }
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
        if self.max_attempts < self.attempts {
            self.max_attempts = self.attempts;
        }

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

/// A new email coming from the in-bound SMTP server
#[derive(Debug)]
pub struct NewMessage {
    pub smtp_credential_id: SmtpCredentialId,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
}

impl NewMessage {
    pub fn new(smtp_credential_id: SmtpCredentialId, from_email: EmailAddress) -> Self {
        NewMessage {
            smtp_credential_id,
            from_email,
            recipients: vec![],
            raw_data: vec![],
            message_data: Default::default(),
        }
    }
}

/// A new email coming from the Remails API
pub struct NewApiMessage {
    pub api_key_id: ApiKeyId,
    pub stream_id: StreamId,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MessageRepository {
    pool: sqlx::PgPool,
    rate_limit_timespan: PgInterval,
    rate_limit_max_messages: i64,
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
    api_key_id: Option<Uuid>,
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
    outbound_ip: Option<IpNet>,
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
            api_key_id: m.api_key_id.map(Into::into),
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
            max_attempts: m.max_attempts,
            outbound_ip: m.outbound_ip.map(|net| net.addr()),
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
            api_key_id: m.api_key_id.map(Into::into),
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
            outbound_ip: m.outbound_ip.map(|net| net.addr()),
        })
    }
}

impl MessageRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        let rate_limit_minutes = std::env::var("RATE_LIMIT_MINUTES")
            .map(|s| s.parse().expect("Invalid RATE_LIMIT_MINUTES"))
            .expect("RATE_LIMIT_MINUTES must be set");
        let rate_limit_max_messages = std::env::var("RATE_LIMIT_MAX_MESSAGES")
            .map(|s| s.parse().expect("Invalid RATE_LIMIT_MAX_MESSAGES"))
            .expect("RATE_LIMIT_MAX_MESSAGES must be set");

        Self {
            pool,
            rate_limit_timespan: PgInterval::try_from(chrono::Duration::minutes(
                rate_limit_minutes,
            ))
            .expect("Could not set rate limit timespan"),
            rate_limit_max_messages,
        }
    }

    pub async fn create(&self, message: &NewMessage, max_attempts: i32) -> Result<Message, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            INSERT INTO messages AS m (id, organization_id, project_id, stream_id, smtp_credential_id, from_email,
                                       recipients, raw_data, message_data, max_attempts, outbound_ip)
            WITH m AS (SELECT o.id AS org_id, p.id AS proj_id, streams.id AS stream_id
                       FROM smtp_credentials s
                                JOIN streams ON s.stream_id = streams.id
                                JOIN projects p ON p.id = streams.project_id
                                JOIN organizations o ON o.id = p.organization_id
                       WHERE s.id = $1),
                 ip AS (SELECT ip AS outbound_ip
                        FROM outbound_ips
                        JOIN k8s_nodes AS node on outbound_ips.node_id = node.id
                        WHERE node.ready
                        -- TODO: Do not rely on random outbound IPs
                        ORDER BY RANDOM()
                        LIMIT 1)
            SELECT gen_random_uuid(), m.org_id, m.proj_id, m.stream_id, $1, $2, $3, $4, $5, $6, ip.outbound_ip
            FROM m, ip
            RETURNING
                m.id,
                m.organization_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.api_key_id,
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
                m.max_attempts,
                m.outbound_ip
            "#,
            *message.smtp_credential_id,
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

    pub async fn create_from_api(
        &self,
        message: &NewApiMessage,
        max_attempts: i32,
    ) -> Result<ApiMessageMetadata, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            INSERT INTO messages AS m (id, organization_id, project_id, stream_id, api_key_id, from_email, recipients, raw_data, max_attempts)
            SELECT gen_random_uuid(), o.id, p.id, $1, $2, $3, $4, $5, $6
            FROM streams s
                JOIN projects p ON p.id = s.project_id
                JOIN organizations o ON o.id = p.organization_id
            WHERE s.id = $1
            RETURNING
                m.id,
                m.organization_id,
                m.project_id,
                m.stream_id,
                m.smtp_credential_id,
                m.api_key_id,
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
                m.max_attempts,
                m.outbound_ip
            "#,
            *message.stream_id,
            *message.api_key_id,
            message.from_email.as_str(),
            &message.recipients.iter().map(|r| r.email()).collect::<Vec<_>>(),
            message.raw_data,
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
                attempts = $6,
                max_attempts = $7
            WHERE id = $1
            "#,
            *message.id,
            message.status as _,
            message.reason,
            delivery_details_serialized,
            message.retry_after,
            message.attempts,
            message.max_attempts,
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
                attempts = $6,
                max_attempts = $7
            WHERE id = $1
            "#,
            *message.id,
            message.message_data,
            message.status as _,
            message.reason,
            message.retry_after,
            message.attempts,
            message.max_attempts,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_message_metadata(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
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
                api_key_id,
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
                max_attempts,
                outbound_ip
            FROM messages m
            WHERE organization_id = $1
                AND project_id = $2
                AND stream_id = $3
                AND ($5::message_status IS NULL OR status = $5)
                AND ($6::timestamptz IS NULL OR created_at <= $6)
            ORDER BY created_at DESC
            LIMIT $4
            "#,
            *org_id,
            *project_id,
            *stream_id,
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
                m.api_key_id,
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
                m.max_attempts,
                m.outbound_ip
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
        project_id: ProjectId,
        stream_id: StreamId,
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
                m.api_key_id,
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
                m.max_attempts,
                m.outbound_ip
            FROM messages m
            WHERE m.id = $1
              AND m.organization_id = $2
              AND m.project_id = $3
              AND m.stream_id = $4
            "#,
            *message_id,
            *org_id,
            *project_id,
            *stream_id,
            API_RAW_TRUNCATE_LENGTH,
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }

    pub async fn remove(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        message_id: MessageId,
    ) -> Result<MessageId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM messages
            WHERE id = $1
              AND organization_id = $2
              AND project_id = $3
              AND stream_id = $4
            RETURNING id
            "#,
            *message_id,
            *org_id,
            *project_id,
            *stream_id,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    /// Messages which should be retried are either:
    /// - on `held` or `reattempt`, not on timeout, with attempts left
    /// - on `accepted` or `processing`, and not having been updated in 15 minutes
    pub async fn find_messages_ready_for_retry(
        &self,
    ) -> Result<Vec<(MessageId, Option<IpAddr>)>, Error> {
        struct IdAndIp {
            id: MessageId,
            outbound_ip: Option<IpNet>,
        }

        Ok(sqlx::query_as!(
            IdAndIp,
            r#"
            SELECT m.id, m.outbound_ip
            FROM messages m
            WHERE (
                (m.status = 'held' OR m.status = 'reattempt')
                AND now() > m.retry_after AND m.attempts < m.max_attempts
              ) OR (
                (m.status = 'accepted' OR m.status = 'processing')
                AND now() > m.updated_at + '2 minutes'
              )
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|elem| (elem.id, elem.outbound_ip.map(|net| net.addr())))
        .collect())
    }

    pub async fn message_status_and_outbound_ip(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        message_id: MessageId,
    ) -> Result<(MessageStatus, Option<IpAddr>), Error> {
        struct StatusAndIp {
            status: MessageStatus,
            outbound_ip: Option<IpNet>,
        }
        let res = sqlx::query_as!(
            StatusAndIp,
            r#"
            SELECT m.status AS "status:MessageStatus", m.outbound_ip
            FROM messages m
            WHERE m.organization_id = $1 
              AND m.project_id = $2 
              AND m.stream_id = $3 
              AND m.id = $4
            "#,
            *org_id,
            *project_id,
            *stream_id,
            *message_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((res.status, res.outbound_ip.map(|net| net.addr())))
    }

    /// Returns the number of emails that can still be created during the current rate limit time span
    ///
    /// Automatically resets when the time span has expired, if so, it starts a new time span
    pub async fn email_creation_rate_limit(&self, id: StreamId) -> Result<i64, Error> {
        let remaining_rate_limit = sqlx::query_scalar!(
            r#"
            UPDATE organizations o
            SET
            remaining_rate_limit = CASE
                WHEN rate_limit_reset < now()
                THEN $2
                ELSE GREATEST(remaining_rate_limit - 1, 0)
            END,
            rate_limit_reset = CASE
                WHEN rate_limit_reset < now()
                THEN now() + $3
                ELSE rate_limit_reset
            END
            FROM streams s JOIN projects p ON p.id = s.project_id
            WHERE p.organization_id = o.id AND s.id = $1
            RETURNING remaining_rate_limit
            "#,
            *id,
            self.rate_limit_max_messages,
            self.rate_limit_timespan
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(remaining_rate_limit)
    }
}

#[cfg(test)]
mod test {
    use mail_builder::MessageBuilder;
    use mail_send::smtp::message::IntoMessage;
    use sqlx::PgPool;

    use super::*;
    use crate::{
        models::{
            ApiKeyRepository, ApiKeyRequest, Role, SmtpCredentialRepository, SmtpCredentialRequest,
        },
        test::TestStreams,
    };

    impl NewMessage {
        pub fn from_builder_message(
            value: mail_send::smtp::message::Message<'_>,
            smtp_credential_id: SmtpCredentialId,
        ) -> Self {
            let mut message = Self::new(smtp_credential_id, value.mail_from.email.parse().unwrap());
            for recipient in value.rcpt_to.iter() {
                message.recipients.push(recipient.email.parse().unwrap());
            }
            message.raw_data = value.body.to_vec();

            message
        }

        pub fn from_builder_message_custom_from(
            value: mail_send::smtp::message::Message<'_>,
            smtp_credential_id: SmtpCredentialId,
            smtp_from: &str,
        ) -> Self {
            let mut message = Self::new(smtp_credential_id, smtp_from.parse().unwrap());
            for recipient in value.rcpt_to.iter() {
                message.recipients.push(recipient.email.parse().unwrap());
            }
            message.raw_data = value.body.to_vec();

            message
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "streams",
            "k8s_nodes"
        )
    ))]
    async fn message_repository(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());
        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

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

        // create SMTP credential
        let smtp_credential_repo = SmtpCredentialRepository::new(pool);
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

        // create message
        let new_message = NewMessage::from_builder_message(message, credential.id());
        let message = repository.create(&new_message, 5).await.unwrap();

        // get message
        let mut fetched_message = repository
            .find_by_id(org_id, project_id, stream_id, message.id)
            .await
            .unwrap();
        assert_eq!(fetched_message.smtp_credential_id(), Some(credential.id()));
        assert_eq!(fetched_message.api_key_id(), None);
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

        // list message metadata
        let messages = repository
            .list_message_metadata(
                org_id,
                project_id,
                stream_id,
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

        // remove message
        repository
            .remove(org_id, project_id, stream_id, message.id)
            .await
            .unwrap();

        // check that message was removed
        let messages = repository
            .list_message_metadata(
                org_id,
                project_id,
                stream_id,
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

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains", "streams")
    ))]
    async fn create_message_from_api(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());
        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

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

        // create API key
        let api_key_repository = ApiKeyRepository::new(pool);
        let api_key = api_key_repository
            .create(
                org_id,
                &ApiKeyRequest {
                    description: "Test API key".to_string(),
                    role: Role::Maintainer,
                },
            )
            .await
            .unwrap();

        // create message
        let new_message = NewApiMessage {
            api_key_id: *api_key.id(),
            stream_id,
            from_email: "john@test-org-1-project-1.com".parse().unwrap(),
            recipients: vec![
                "james@test.com".parse().unwrap(),
                "jane@test-org-1-project-1.com".parse().unwrap(),
            ],
            raw_data: message.into_message().unwrap().body.to_vec(),
        };
        let message = repository.create_from_api(&new_message, 5).await.unwrap();

        // get message
        let mut fetched_message = repository
            .find_by_id(org_id, project_id, stream_id, message.id)
            .await
            .unwrap();
        assert_eq!(fetched_message.smtp_credential_id(), None);
        assert_eq!(fetched_message.api_key_id(), Some(*api_key.id()));
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
    }
}
