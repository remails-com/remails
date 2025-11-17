use crate::{
    bus::client::BusMessage,
    handler::{ConnectionLog, RetryConfig},
    models::{
        ApiKeyId, Error, OrgBlockStatus, OrganizationId, SmtpCredentialId, projects::ProjectId,
    },
};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use garde::Validate;
use mail_parser::MimeHeaders;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgInterval;
use std::{collections::HashMap, mem, str::FromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

const API_RAW_TRUNCATE_LENGTH: i32 = 10_000;

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, ToSchema,
)]
pub struct MessageId(Uuid);

impl MessageId {
    pub fn new_v4() -> Self {
        MessageId(Uuid::new_v4())
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Serialize, sqlx::Type, Display, ToSchema)]
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
    pub(crate) smtp_credential_id: Option<SmtpCredentialId>,
    pub(crate) api_key_id: Option<ApiKeyId>,
    pub status: MessageStatus,
    pub reason: Option<String>,
    pub delivery_details: HashMap<EmailAddress, DeliveryDetails>,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    pub raw_data: Vec<u8>,
    pub message_data: serde_json::Value,
    pub message_id_header: Option<String>,
    pub created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    pub retry_after: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub max_attempts: i32,
}

#[derive(Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ApiMessage {
    #[serde(flatten)]
    metadata: ApiMessageMetadata,
    pub truncated_raw_data: String,
    /// Indicates if the `truncated_raw_data` are actually truncated.
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
}

#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize, ToSchema)]
pub struct ApiMessageMetadata {
    pub id: MessageId,
    pub status: MessageStatus,
    reason: Option<String>,
    /// Delivery details for each recipient Remails tried to deliver to already.
    /// Uses the recipient email as key and `DeliveryDetails` as value.
    delivery_details: HashMap<EmailAddress, DeliveryDetails>,
    pub smtp_credential_id: Option<SmtpCredentialId>,
    pub api_key_id: Option<ApiKeyId>,
    pub from_email: EmailAddress,
    pub recipients: Vec<EmailAddress>,
    /// Human-readable size
    raw_size: String,
    pub message_id_header: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    retry_after: Option<DateTime<Utc>>,
    #[schema(minimum = 0)]
    attempts: i32,
    #[schema(minimum = 0)]
    max_attempts: i32,
}

#[derive(Serialize, Default, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ApiMessageData {
    pub subject: Option<String>,
    /// An RFC3339 String
    pub date: Option<String>,
    pub text_body: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct Attachment {
    pub filename: String,
    pub mime: String,
    /// Human-readable size
    pub size: String,
}

#[derive(Debug, Deserialize, Serialize, Default, ToSchema)]
#[serde(tag = "type")]
pub enum DeliveryStatus {
    #[default]
    #[schema(title = "None")]
    None,
    #[schema(title = "Success")]
    Success { delivered: DateTime<Utc> },
    #[schema(title = "Reattempt")]
    Reattempt,
    #[schema(title = "Failed")]
    Failed,
}

/// Details of the email transmission for a specific recipient
#[derive(Debug, Deserialize, Serialize, Default, ToSchema)]
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
    pub message_id: MessageId,
    pub message_id_header: String,
    pub api_key_id: ApiKeyId,
    pub project_id: ProjectId,
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

#[derive(Debug, Deserialize, IntoParams, Validate)]
#[serde(default)]
pub struct MessageFilter {
    #[param(minimum = 1, maximum = 100, default = 10)]
    #[garde(range(min = 1, max = 100))]
    limit: i64,
    #[garde(skip)]
    status: Option<MessageStatus>,
    #[garde(skip)]
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
    message_id_header: Option<String>,
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
            message_id_header: m.message_id_header,
            created_at: m.created_at,
            updated_at: m.updated_at,
            retry_after: m.retry_after,
            attempts: m.attempts,
            max_attempts: m.max_attempts,
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
            message_id_header: m.message_id_header,
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

    pub async fn get_ready_to_send(&self, message_id: MessageId) -> Result<BusMessage, Error> {
        // TODO: do not rely on random outbound IPs
        match sqlx::query_scalar!(
            r#"
            SELECT ip AS outbound_ip
            FROM outbound_ips
            JOIN k8s_nodes AS node on outbound_ips.node_id = node.id
            JOIN messages m ON m.id = $1
            JOIN organizations o ON o.id = m.organization_id
            WHERE node.ready AND o.block_status = 'not_blocked'
            ORDER BY RANDOM()
            LIMIT 1
            "#,
            *message_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(outbound_ip)) => {
                Ok(BusMessage::EmailReadyToSend(message_id, outbound_ip.addr()))
            }
            Ok(None) => Err(Error::Internal(
                "failed to assign outbound IP to message: none available".to_string(),
            )),
            Err(e) => Err(Error::Internal(format!(
                "failed to assign outbound IP to message: {e:?}"
            ))),
        }
    }

    /// Generate a unique message ID to be included as email header in case no message ID was provided
    pub fn generate_message_id_header(id: &MessageId, from_email: &EmailAddress) -> String {
        let sender_domain = from_email.domain();

        // including the Remails message UUID ensure uniqueness
        format!("REMAILS-{id}@{sender_domain}")
    }

    pub async fn create(
        &self,
        message: &NewMessage,
        max_attempts: i32,
    ) -> Result<MessageId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            INSERT INTO messages AS m (
                id, organization_id, project_id, smtp_credential_id,
                from_email, recipients, raw_data, message_data, max_attempts
            )
            SELECT gen_random_uuid(), o.id, p.id, $1, $2, $3, $4, $5, $6
            FROM smtp_credentials s
                JOIN projects p ON p.id = s.project_id
                JOIN organizations o ON o.id = p.organization_id
            WHERE s.id = $1
            RETURNING
                m.id
            "#,
            *message.smtp_credential_id,
            message.from_email.as_str(),
            &message
                .recipients
                .iter()
                .map(|r| r.email())
                .collect::<Vec<_>>(),
            message.raw_data,
            message.message_data,
            max_attempts,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    pub async fn create_from_api(
        &self,
        message: &NewApiMessage,
        max_attempts: i32,
    ) -> Result<ApiMessageMetadata, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            INSERT INTO messages AS m (
                id, organization_id, project_id, api_key_id,
                from_email, recipients, raw_data, max_attempts, message_id_header
            )
            SELECT $1, o.id, $2, $3, $4, $5, $6, $7, $8
            FROM projects p
                JOIN organizations o ON o.id = p.organization_id
            WHERE p.id = $2
            RETURNING
                m.id,
                m.organization_id,
                m.project_id,
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
                m.message_id_header,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            "#,
            *message.message_id,
            *message.project_id,
            *message.api_key_id,
            message.from_email.as_str(),
            &message
                .recipients
                .iter()
                .map(|r| r.email())
                .collect::<Vec<_>>(),
            message.raw_data,
            max_attempts,
            message.message_id_header
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
                max_attempts = $7,
                message_id_header = $8
            WHERE id = $1
            "#,
            *message.id,
            message.message_data,
            message.status as _,
            message.reason,
            message.retry_after,
            message.attempts,
            message.max_attempts,
            message.message_id_header,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_message_metadata(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        filter: MessageFilter,
    ) -> Result<Vec<ApiMessageMetadata>, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                id,
                organization_id,
                project_id,
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
                message_id_header,
                created_at,
                updated_at,
                retry_after,
                attempts,
                max_attempts
            FROM messages m
            WHERE organization_id = $1
                AND project_id = $2
                AND ($4::message_status IS NULL OR status = $4)
                AND ($5::timestamptz IS NULL OR created_at <= $5)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            *org_id,
            *project_id,
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

    /// Get a specific message, but only if the organization is allowed to send
    ///
    /// Unlike [`find_by_id`] this returns a `Message` with the full raw data
    pub async fn get_if_org_may_send(&self, message_id: MessageId) -> Result<Message, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.project_id,
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
                m.message_id_header,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            FROM messages m
            JOIN organizations o ON o.id = m.organization_id
            WHERE m.id = $1 AND o.block_status = 'not_blocked'
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
        message_id: MessageId,
    ) -> Result<ApiMessage, Error> {
        sqlx::query_as!(
            PgMessage,
            r#"
            SELECT
                m.id,
                m.organization_id,
                m.project_id,
                m.smtp_credential_id,
                m.api_key_id,
                m.status as "status: _",
                m.reason,
                m.delivery_details,
                m.from_email,
                m.recipients,
                -- Only return the first API_RAW_TRUNCATE_LENGTH bytes/ASCII-characters of the raw data.
                substring(m.raw_data FOR $4) as "raw_data!",
                octet_length(m.raw_data) as "raw_size!",
                m.message_data,
                m.message_id_header,
                m.created_at,
                m.updated_at,
                m.retry_after,
                m.attempts,
                m.max_attempts
            FROM messages m
            WHERE m.id = $1
              AND m.organization_id = $2
              AND m.project_id = $3
            "#,
            *message_id,
            *org_id,
            *project_id,
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
        message_id: MessageId,
    ) -> Result<MessageId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM messages
            WHERE id = $1
              AND organization_id = $2
              AND project_id = $3
            RETURNING id
            "#,
            *message_id,
            *org_id,
            *project_id,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    /// Messages which should be retried are either:
    ///
    /// - on `held` or `reattempt`, not on timeout, with attempts left
    /// - on `accepted` or `processing`, and not having been updated in 2 minutes
    ///
    /// and the organization must be allowed to send messages (must not be blocked)
    pub async fn find_messages_ready_for_retry(&self) -> Result<Vec<MessageId>, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT m.id FROM messages m
            JOIN organizations o ON o.id = m.organization_id
            WHERE o.block_status = 'not_blocked' AND (
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
        .map(Into::into)
        .collect())
    }

    pub async fn message_status(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        message_id: MessageId,
    ) -> Result<MessageStatus, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT m.status AS "status:MessageStatus"
            FROM messages m
            WHERE m.organization_id = $1 
              AND m.project_id = $2
              AND m.id = $3
            "#,
            *org_id,
            *project_id,
            *message_id,
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// Returns the number of emails that can still be created during the current rate limit time span
    ///
    /// Automatically resets when the time span has expired, if so, it starts a new time span
    ///
    /// Also checks if the organization is allowed to receive new emails (is not blocked)
    pub async fn email_creation_rate_limit(&self, id: ProjectId) -> Result<i64, Error> {
        let result = sqlx::query!(
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
            FROM projects p
            WHERE p.organization_id = o.id
              AND p.id = $1
            RETURNING remaining_rate_limit, o.block_status as "block_status: OrgBlockStatus"
            "#,
            *id,
            self.rate_limit_max_messages,
            self.rate_limit_timespan
        )
        .fetch_one(&self.pool)
        .await?;

        if result.block_status == OrgBlockStatus::NoSendingOrReceiving {
            return Err(Error::OrgBlocked);
        }

        Ok(result.remaining_rate_limit)
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
            ApiKeyRepository, ApiKeyRequest, OrganizationRepository, Role,
            SmtpCredentialRepository, SmtpCredentialRequest,
        },
        test::TestProjects,
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
            "k8s_nodes"
        )
    ))]
    async fn message_repository(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());
        let (org_id, project_id) = TestProjects::Org1Project1.get_ids();

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
                &SmtpCredentialRequest {
                    username: "user".to_string(),
                    description: "Test SMTP credential description".to_string(),
                },
            )
            .await
            .unwrap();

        // create message
        let new_message = NewMessage::from_builder_message(message, credential.id());
        let message_id = repository.create(&new_message, 5).await.unwrap();

        // get message
        let mut fetched_message = repository
            .find_by_id(org_id, project_id, message_id)
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
                MessageFilter {
                    limit: 5,
                    status: None,
                    before: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, message_id);

        // remove message
        repository
            .remove(org_id, project_id, message_id)
            .await
            .unwrap();

        // check that message was removed
        let messages = repository
            .list_message_metadata(
                org_id,
                project_id,
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
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn create_message_from_api(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());
        let (org_id, project_id) = TestProjects::Org1Project1.get_ids();

        let from = "john@test-org-1-project-1.com";
        let message_id = MessageId::new_v4();
        let message_id_header =
            MessageRepository::generate_message_id_header(&message_id, &from.parse().unwrap());
        let message = MessageBuilder::new()
            .from(("John Doe", "john@test-org-1-project-1.com"))
            .to(vec![
                ("James Smith", "james@test.com"),
                ("Jane Doe", "jane@test-org-1-project-1.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .message_id(message_id_header.as_str())
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
            message_id,
            message_id_header,
            api_key_id: *api_key.id(),
            project_id,
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
            .find_by_id(org_id, project_id, message.id)
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

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "messages_length_truncation")
    ))]
    async fn truncation(pool: PgPool) {
        let repository = MessageRepository::new(pool.clone());

        let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let proj_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let message_just_fits = "525f7d40-cb6d-402b-9078-0275c22808d7".parse().unwrap();
        let message_just_too_log = "c1d2c3d6-1521-4f77-804a-2034d121c9b0".parse().unwrap();

        let message = repository
            .find_by_id(org_id, proj_id, message_just_fits)
            .await
            .unwrap();
        assert!(!message.is_truncated);
        assert_eq!(
            message.truncated_raw_data,
            format!("{}Y", "x".repeat((API_RAW_TRUNCATE_LENGTH - 1) as usize))
        );

        let message = repository
            .find_by_id(org_id, proj_id, message_just_too_log)
            .await
            .unwrap();
        assert!(message.is_truncated);
        assert_eq!(
            message.truncated_raw_data,
            "x".repeat(API_RAW_TRUNCATE_LENGTH as usize)
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "smtp_credentials",
            "messages"
        )
    ))]
    async fn test_blocked_checks(pool: PgPool) {
        let organizations = OrganizationRepository::new(pool.clone());
        let messages = MessageRepository::new(pool.clone());

        let (org_id, proj_id) = TestProjects::Org1Project1.get_ids();
        let message_id = "e165562a-fb6d-423b-b318-fd26f4610634".parse().unwrap();

        // org 1 starts out as Not Blocked
        let message = messages.get_if_org_may_send(message_id).await.unwrap(); // can send
        assert_eq!(message.id(), message_id);

        messages.email_creation_rate_limit(proj_id).await.unwrap(); // can receive

        // set org 1 to No Sending
        organizations
            .update_block_status(org_id, OrgBlockStatus::NoSending)
            .await
            .unwrap();

        let err = messages.get_if_org_may_send(message_id).await.unwrap_err(); // can't send
        assert!(matches!(err, Error::NotFound(_)));

        messages.email_creation_rate_limit(proj_id).await.unwrap(); // can receive

        // set org 1 to No Sending Or Receiving
        organizations
            .update_block_status(org_id, OrgBlockStatus::NoSendingOrReceiving)
            .await
            .unwrap();

        let err = messages.get_if_org_may_send(message_id).await.unwrap_err(); // can't send
        assert!(matches!(err, Error::NotFound(_)));

        let err = messages
            .email_creation_rate_limit(proj_id)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::OrgBlocked)); // can't receive

        // reset org 1 to Not Blocked
        organizations
            .update_block_status(org_id, OrgBlockStatus::NotBlocked)
            .await
            .unwrap();

        let message = messages.get_if_org_may_send(message_id).await.unwrap(); // can send again
        assert_eq!(message.id(), message_id);

        messages.email_creation_rate_limit(proj_id).await.unwrap(); // can receive again
    }
}
