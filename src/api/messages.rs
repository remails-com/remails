use std::sync::Arc;

use super::error::{ApiError, ApiResult};
use crate::{
    api::auth::Authenticated,
    bus::client::BusClient,
    handler::RetryConfig,
    models::{
        ApiKey, ApiMessage, ApiMessageMetadata, MessageFilter, MessageId, MessageRepository,
        MessageStatus, NewApiMessage, OrganizationId, ProjectId, StreamId,
    },
};
use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use http::StatusCode;
use mail_builder::MessageBuilder;
use serde::Deserialize;
use tracing::{debug, error, warn};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmailAddress {
    AddressOnly(String),
    WithName { name: String, address: String },
}

impl EmailAddress {
    fn get_mail_address(&self) -> &String {
        match self {
            EmailAddress::AddressOnly(address) => address,
            EmailAddress::WithName { address, .. } => address,
        }
    }
}

impl<'a> From<EmailAddress> for mail_builder::headers::address::Address<'a> {
    fn from(address: EmailAddress) -> Self {
        match address {
            EmailAddress::AddressOnly(address) => address.into(),
            EmailAddress::WithName { name, address } => (name, address).into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmailAddresses {
    Singular(EmailAddress),
    Multiple(Vec<EmailAddress>),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailParameters {
    from: EmailAddress,
    to: EmailAddresses,
    subject: String,
    text_body: Option<String>,
    html_body: Option<String>,
    in_reply_to: Option<String>,
    references: Option<Vec<String>>,
    reply_to: Option<EmailAddress>,
}

impl<'a> From<EmailAddresses> for mail_builder::headers::address::Address<'a> {
    fn from(addresses: EmailAddresses) -> Self {
        match addresses {
            EmailAddresses::Singular(a) => a.into(),
            EmailAddresses::Multiple(a) => a.into(),
        }
    }
}

pub async fn create_message(
    State(repo): State<MessageRepository>,
    State(retry_config): State<Arc<RetryConfig>>,
    State(bus_client): State<Arc<BusClient>>,
    Path((org_id, _, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    key: ApiKey, // only accessible for API keys
    Json(message): Json<EmailParameters>,
) -> Result<impl IntoResponse, ApiError> {
    key.has_org_write_access(&org_id)?;

    // check email rate limit
    if repo.email_creation_rate_limit(stream_id).await? <= 0 {
        debug!("too many email requests for org {org_id}");
        return Err(ApiError::TooManyRequests);
    }

    // parse from email
    let from_email = message.from.get_mail_address();
    let from_email = from_email
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("Invalid from email: {}", from_email)))?;

    // parse recipient's email(s)
    let recipients =
        match &message.to {
            EmailAddresses::Singular(to) => {
                let address = to.get_mail_address();
                vec![address.parse().map_err(|_| {
                    ApiError::BadRequest(format!("Invalid recipient email: {address}"))
                })?]
            }
            EmailAddresses::Multiple(to) => {
                let mut recipients = Vec::with_capacity(to.len());
                for recipient in to {
                    let address = recipient.get_mail_address();
                    recipients.push(address.parse().map_err(|_| {
                        ApiError::BadRequest(format!("Invalid recipient email: {address}"))
                    })?);
                }
                recipients
            }
        };
    if recipients.is_empty() {
        return Err(ApiError::BadRequest(
            "Must have at least one recipient".to_owned(),
        ));
    }

    // generate message ID
    let message_id = MessageId::new_v4();
    let message_id_header = MessageRepository::generate_message_id_header(&message_id, &from_email);

    // set required fields
    let mut message_builder = MessageBuilder::new()
        .from(message.from)
        .to(message.to)
        .subject(message.subject)
        .message_id(message_id_header.as_str());

    // add body to message
    if message.text_body.is_none() && message.html_body.is_none() {
        return Err(ApiError::BadRequest(
            "Must provide a text_body or html_body".to_owned(),
        ));
    }
    if let Some(text_body) = message.text_body {
        message_builder = message_builder.text_body(text_body)
    }
    if let Some(html_body) = message.html_body {
        message_builder = message_builder.html_body(html_body);
    }

    // add optional headers
    if let Some(in_reply_to) = message.in_reply_to {
        message_builder = message_builder.in_reply_to(in_reply_to);
    }
    if let Some(references) = message.references {
        message_builder = message_builder.references(references);
    }
    if let Some(reply_to) = message.reply_to {
        message_builder = message_builder.reply_to(reply_to);
    }

    let raw_data = message_builder
        .write_to_vec()
        .map_err(|e| ApiError::BadRequest(format!("Error creating email: {e:?}")))?;

    let message = NewApiMessage {
        message_id,
        message_id_header,
        api_key_id: *key.id(),
        stream_id,
        from_email,
        recipients,
        raw_data,
    };

    debug!(
        organization_id = org_id.to_string(),
        stream_id = stream_id.to_string(),
        message_id = message_id.to_string(),
        api_key_id = key.id().to_string(),
        "creating message from API"
    );

    let message = repo
        .create_from_api(&message, retry_config.max_automatic_retries)
        .await?;

    match repo.get_ready_to_send(message.id).await {
        Ok(bus_message) => {
            bus_client.try_send(&bus_message).await;
        }
        Err(e) => {
            error!(message_id = message.id.to_string(), "{e:?}");
        }
    }

    Ok((StatusCode::CREATED, Json(message)))
}

pub async fn list_messages(
    State(repo): State<MessageRepository>,
    Path((org_id, project_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    Query(filter): Query<MessageFilter>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Vec<ApiMessageMetadata>> {
    user.has_org_read_access(&org_id)?;

    let messages = repo
        .list_message_metadata(org_id, project_id, stream_id, filter)
        .await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.to_string(),
        stream_id = stream_id.to_string(),
        "listed {} messages",
        messages.len()
    );

    Ok(Json(messages))
}

pub async fn get_message(
    State(repo): State<MessageRepository>,
    Path((org_id, project_id, stream_id, message_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        MessageId,
    )>,
    user: Box<dyn Authenticated>,
) -> ApiResult<ApiMessage> {
    user.has_org_read_access(&org_id)?;

    let message = repo
        .find_by_id(org_id, project_id, stream_id, message_id)
        .await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.to_string(),
        stream_id = stream_id.to_string(),
        message_id = message_id.to_string(),
        "retrieved message",
    );

    Ok(Json(message))
}

pub async fn remove_message(
    State(repo): State<MessageRepository>,
    Path((org_id, project_id, stream_id, message_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        MessageId,
    )>,
    user: Box<dyn Authenticated>,
) -> ApiResult<()> {
    user.has_org_write_access(&org_id)?;

    repo.remove(org_id, project_id, stream_id, message_id)
        .await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.to_string(),
        stream_id = stream_id.to_string(),
        message_id = message_id.to_string(),
        "removed message",
    );

    Ok(Json(()))
}

pub async fn retry_now(
    State(repo): State<MessageRepository>,
    State(bus_client): State<Arc<BusClient>>,
    Path((org_id, project_id, stream_id, message_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        MessageId,
    )>,
    user: Box<dyn Authenticated>,
) -> Result<(), ApiError> {
    user.has_org_write_access(&org_id)?;

    let status = repo
        .message_status(org_id, project_id, stream_id, message_id)
        .await?;

    if status == MessageStatus::Delivered {
        warn!(
            message_id = message_id.to_string(),
            user_id = user.log_id(),
            "Requested retry for already delivered message"
        );
        return Err(ApiError::BadRequest(
            "Message already delivered".to_string(),
        ));
    }

    match repo.get_ready_to_send(message_id).await {
        Ok(bus_message) => {
            bus_client.try_send(&bus_message).await;
        }
        Err(e) => {
            error!(message_id = message_id.to_string(), "{e:?}");
        }
    }

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.to_string(),
        stream_id = stream_id.to_string(),
        message_id = message_id.to_string(),
        "requested message retry",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        bus::client::BusMessage,
        models::{MessageStatus, Role},
        test::TestStreams,
    };
    use axum::body::Body;
    use futures::StreamExt;
    use http::StatusCode;
    use ppp::v2::WriteToHeader;
    use serde_json::json;
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages",
            "k8s_nodes"
        )
    ))]
    async fn test_messages_lifecycle(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let (org_1, proj_1, stream_1) = TestStreams::Org1Project1Stream1.get_ids();
        let server = TestServer::new(pool.clone(), Some(user_1)).await;
        let mut message_stream = server.message_bus.receive().await.unwrap();

        // list messages
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let messages: Vec<ApiMessageMetadata> = deserialize_body(response.into_body()).await;
        let messages_in_fixture = 5;
        assert_eq!(messages.len(), messages_in_fixture);

        // get specific message
        let message_1 = "e165562a-fb6d-423b-b318-fd26f4610634";
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let message: ApiMessage = deserialize_body(response.into_body()).await;
        assert_eq!(*message.status(), MessageStatus::Processing);
        assert_eq!(message.id().to_string(), message_1);

        // update message to retry asap
        let response = server
            .put(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}/retry"
            ), Body::empty())
            .await
            .unwrap();
        let status = response.status();
        let server_response = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap()
                .to_bytes()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(status, StatusCode::OK, "server response: {server_response}");

        let bus_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            bus_message,
            BusMessage::EmailReadyToSend(message.id(), "127.0.0.1".parse().unwrap())
        );

        // remove message
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // check if message is deleted
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let messages: Vec<ApiMessageMetadata> = deserialize_body(response.into_body()).await;
        assert_eq!(messages.len(), messages_in_fixture - 1);
        assert!(!messages.iter().any(|m| m.id == message_1.parse().unwrap()));
    }

    async fn test_messages_no_access(
        server: TestServer,
        read_status_code: StatusCode,
        write_status_code: StatusCode,
    ) {
        let (org_1, proj_1, stream_1) = TestStreams::Org1Project1Stream1.get_ids();

        // can't list messages
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't get specific message
        let message_1 = "e165562a-fb6d-423b-b318-fd26f4610634";
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't update message to retry asap
        let response = server
            .put(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}/retry"
            ), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't remove message
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages"
        )
    ))]
    async fn test_messages_no_access_wrong_user(pool: PgPool) {
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let server = TestServer::new(pool.clone(), Some(user_2)).await;
        test_messages_no_access(server, StatusCode::FORBIDDEN, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages"
        )
    ))]
    async fn test_messages_no_access_read_only(pool: PgPool) {
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e".parse().unwrap(); // is read only in org 1
        let server = TestServer::new(pool.clone(), Some(user_5)).await;
        test_messages_no_access(server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages"
        )
    ))]
    async fn test_messages_no_access_not_logged_in(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;
        test_messages_no_access(server, StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_create_message(pool: PgPool) {
        let (org_1, proj_1, stream_1) = TestStreams::Org1Project1Stream1.get_ids();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), Some(user_4)).await;
        let api_key_id = server.use_api_key(org_1, Role::Maintainer).await;

        // send email with 1 recipient, text and HTML body, `in_reply_to`, `references` and `reply_to`
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": "test@example.com",
                    "to": "recipient@example.com",
                    "subject": "subject",
                    "text_body": "text body",
                    "html_body": "<h1>html body</h1>",
                    "in_reply_to": "some-message@example.com",
                    "references": ["some-message@example.com", "some-other-message@example.com"],
                    "reply_to": "support@example.com",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let message: ApiMessageMetadata = deserialize_body(response.into_body()).await;
        assert_eq!(message.from_email.as_str(), "test@example.com");
        assert_eq!(message.recipients.len(), 1);
        assert_eq!(message.recipients[0].as_str(), "recipient@example.com");
        assert_eq!(message.smtp_credential_id, None);
        assert_eq!(message.api_key_id, Some(api_key_id));
        assert_eq!(
            message.message_id_header,
            Some(format!("REMAILS-{}@example.com", message.id))
        );

        // send email with 2 recipients, only text body, and custom from name
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": {"name": "Test", "address": "test@example.com"},
                    "to": ["recipient1@example.com", "recipient2@example.com"],
                    "subject": "subject",
                    "text_body": "text body",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let message: ApiMessageMetadata = deserialize_body(response.into_body()).await;
        assert_eq!(message.from_email.as_str(), "test@example.com");
        let mut recipients = message
            .recipients
            .into_iter()
            .map(|e| e.as_str().to_owned())
            .collect::<Vec<_>>();
        recipients.sort();
        assert_eq!(
            recipients,
            vec!["recipient1@example.com", "recipient2@example.com"]
        );

        // send email with 3 recipients, only HTML body
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": "test@example.com",
                    "to": ["recipient1@example.com", "recipient2@example.com", {"name": "Recipient 3", "address": "recipient3@example.com"}],
                    "subject": "subject",
                    "text_body": "text body",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let message: ApiMessageMetadata = deserialize_body(response.into_body()).await;
        assert_eq!(message.from_email.as_str(), "test@example.com");
        let mut recipients = message
            .recipients
            .into_iter()
            .map(|e| e.as_str().to_owned())
            .collect::<Vec<_>>();
        recipients.sort();
        assert_eq!(
            recipients,
            vec![
                "recipient1@example.com",
                "recipient2@example.com",
                "recipient3@example.com"
            ]
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_create_message_reject(pool: PgPool) {
        let (org_1, proj_1, stream_1) = TestStreams::Org1Project1Stream1.get_ids();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), Some(user_4)).await;
        server.use_api_key(org_1, Role::Maintainer).await;

        // reject emails without body
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": "test@example.com",
                    "to": "recipient@example.com",
                    "subject": "subject",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // reject emails without recipients
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": "test@example.com",
                    "to": [],
                    "subject": "subject",
                    "text_body": "text body",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // reject emails with invalid from email
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(json!({
                    "from": "remails.net",
                    "to": "recipient@example.com",
                    "subject": "subject",
                    "text_body": "text body",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_create_message_no_access(pool: PgPool) {
        let (org_1, proj_1, stream_1) = TestStreams::Org1Project1Stream1.get_ids();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), None).await;

        let message_request = json!({
            "from": "test@example.com",
            "to": "recipient@example.com",
            "subject": "subject",
            "text_body": "text body",
        });
        let try_post = |server: &TestServer| {
            server.post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"),
                serialize_body(message_request.clone()),
            )
        };

        // not logged-in user cannot create emails
        let response = try_post(&server).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // logged-in user also cannot create emails (only API keys can)
        server.set_user(Some(user_4));
        let response = try_post(&server).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // read-only API keys cannot create emails
        server.use_api_key(org_1, Role::ReadOnly).await;
        let response = try_post(&server).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    // TODO: test messages from the API are attempted to be delivered immediately
    // maybe test this in test.rs?
}
