use super::error::{ApiError, ApiResult};
use crate::models::{
    ApiMessage, ApiMessageMetadata, ApiUser, MessageFilter, MessageId, MessageRepository,
    MessageRetryUpdate, OrganizationId, ProjectId, StreamId,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use tracing::debug;

fn has_read_access(
    org: &OrganizationId,
    proj: Option<&ProjectId>,
    stream: Option<&StreamId>,
    message: Option<&MessageId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, proj, stream, message, user)
}

fn has_write_access(
    org: &OrganizationId,
    _proj: Option<&ProjectId>,
    _stream: Option<&StreamId>,
    _message: Option<&MessageId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.is_org_admin(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

#[derive(Debug, Deserialize)]
pub struct MessagePath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
    stream_id: Option<StreamId>,
}

#[derive(Debug, Deserialize)]
pub struct SpecificMessagePath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
    stream_id: Option<StreamId>,
    message_id: MessageId,
}

pub async fn list_messages(
    State(repo): State<MessageRepository>,
    Path(MessagePath {
        org_id,
        project_id,
        stream_id,
    }): Path<MessagePath>,
    Query(filter): Query<MessageFilter>,
    user: ApiUser,
) -> ApiResult<Vec<ApiMessageMetadata>> {
    has_read_access(
        &org_id,
        project_id.as_ref(),
        stream_id.as_ref(),
        None,
        &user,
    )?;

    let messages = repo
        .list_message_metadata(org_id, project_id, stream_id, filter)
        .await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        stream_id = stream_id.map(|id| id.to_string()),
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
    user: ApiUser,
) -> ApiResult<ApiMessage> {
    has_read_access(
        &org_id,
        project_id.as_ref(),
        stream_id.as_ref(),
        Some(&message_id),
        &user,
    )?;

    let message = repo
        .find_by_id(org_id, project_id, stream_id, message_id)
        .await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        stream_id = stream_id.map(|id| id.to_string()),
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
    user: ApiUser,
) -> ApiResult<()> {
    has_write_access(
        &org_id,
        project_id.as_ref(),
        stream_id.as_ref(),
        Some(&message_id),
        &user,
    )?;

    repo.remove(org_id, project_id, stream_id, message_id)
        .await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        stream_id = stream_id.map(|id| id.to_string()),
        message_id = message_id.to_string(),
        "removed message",
    );

    Ok(Json(()))
}

pub async fn update_to_retry_asap(
    State(repo): State<MessageRepository>,
    Path(SpecificMessagePath {
        org_id,
        project_id,
        stream_id,
        message_id,
    }): Path<SpecificMessagePath>,
    user: ApiUser,
) -> ApiResult<MessageRetryUpdate> {
    has_write_access(
        &org_id,
        project_id.as_ref(),
        stream_id.as_ref(),
        Some(&message_id),
        &user,
    )?;

    let update = repo
        .update_to_retry_asap(org_id, project_id, stream_id, message_id)
        .await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        stream_id = stream_id.map(|id| id.to_string()),
        message_id = message_id.to_string(),
        "updated message to retry asap",
    );

    Ok(axum::Json(update))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::StatusCode;
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body},
        models::MessageStatus,
    };

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
        let server = TestServer::new(pool.clone(), user_1).await;

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
        assert_eq!(response.status(), StatusCode::OK);
        let _: MessageRetryUpdate = deserialize_body(response.into_body()).await;

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
    async fn test_messages_no_access(pool: PgPool) {
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1
        let server = TestServer::new(pool.clone(), user_2).await;

        // can't list messages
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't get specific message
        let message_1 = "e165562a-fb6d-423b-b318-fd26f4610634";
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't update message to retry asap
        let response = server
            .put(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}/retry"
            ), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't remove message
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
