use std::sync::Arc;

use super::error::{ApiError, ApiResult};
use crate::{
    api::auth::Authenticated,
    bus::client::{BusClient, BusMessage},
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
pub struct MessagePath {
    org_id: OrganizationId,
    project_id: ProjectId,
    stream_id: StreamId,
}

#[derive(Debug, Deserialize)]
pub struct SpecificMessagePath {
    org_id: OrganizationId,
    project_id: ProjectId,
    stream_id: StreamId,
    message_id: MessageId,
}

#[derive(Debug, Deserialize)]
pub struct EmailParameters {
    from: String,
    to: Vec<String>,
    subject: String,
    text_body: String,
    html_body: String,
}

pub async fn create_message(
    State(repo): State<MessageRepository>,
    Path((org_id, _, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    user: ApiKey, // only accessible for API keys
    Json(message): Json<EmailParameters>,
) -> Result<impl IntoResponse, ApiError> {
    user.has_org_write_access(&org_id)?;

    // TODO: rate limiting

    let from_email = message
        .from
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("Invalid from email: {}", message.from)))?;

    let mut recipients = Vec::with_capacity(message.to.len());
    for recipient in &message.to {
        recipients.push(
            recipient.parse().map_err(|_| {
                ApiError::BadRequest(format!("Invalid recipient email: {recipient}"))
            })?,
        );
    }

    let raw_data = MessageBuilder::new()
        .from(message.from)
        .to(message.to)
        .subject(message.subject)
        .text_body(message.text_body)
        .html_body(message.html_body)
        .write_to_vec()
        .map_err(|e| ApiError::BadRequest(format!("Error creating email: {e:?}")))?;

    let message = NewApiMessage {
        api_key_id: *user.id(),
        stream_id,
        from_email,
        recipients,
        raw_data,
    };

    let message = repo
        .create_from_api(&message, RetryConfig::default().max_automatic_retries)
        .await?;

    // TODO: do basic checks immediately and return an error if it fails?

    Ok((StatusCode::CREATED, Json(message)))
}

pub async fn list_messages(
    State(repo): State<MessageRepository>,
    Path(MessagePath {
        org_id,
        project_id,
        stream_id,
    }): Path<MessagePath>,
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
    Path(SpecificMessagePath {
        org_id,
        project_id,
        stream_id,
        message_id,
    }): Path<SpecificMessagePath>,
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
    Path(SpecificMessagePath {
        org_id,
        project_id,
        stream_id,
        message_id,
    }): Path<SpecificMessagePath>,
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
    State(bus): State<Arc<BusClient>>,
    Path(SpecificMessagePath {
        org_id,
        project_id,
        stream_id,
        message_id,
    }): Path<SpecificMessagePath>,
    user: Box<dyn Authenticated>,
) -> Result<(), ApiError> {
    user.has_org_write_access(&org_id)?;

    let (status, Some(outbound_ip)) = repo
        .message_status_and_outbound_ip(org_id, project_id, stream_id, message_id)
        .await?
    else {
        error!("Requested retry for message that doesn't have an outbound IP assigned (yet)");
        return Err(ApiError::BadRequest(
            "Message doesn't have an outbound IP assigned (yet)".to_string(),
        ));
    };

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

    bus.send(&BusMessage::EmailReadyToSend(message_id, outbound_ip))
        .await
        .map_err(ApiError::MessageBus)?;

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
    use crate::{
        api::tests::{TestServer, deserialize_body},
        models::MessageStatus,
    };
    use axum::body::Body;
    use futures::StreamExt;
    use http::StatusCode;
    use ppp::v2::WriteToHeader;
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
            "messages"
        )
    ))]
    async fn test_messages_lifecycle(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1
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
        let bus_message = message_stream.next().await.unwrap();
        assert_eq!(
            bus_message,
            BusMessage::EmailReadyToSend(message.id(), message.outbound_ip().unwrap())
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
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1

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
}
