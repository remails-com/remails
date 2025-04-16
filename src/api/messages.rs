use super::error::{ApiError, ApiResult};
use crate::models::{
    ApiUser, Message, MessageFilter, MessageId, MessageRepository, OrganizationId, ProjectId,
    StreamId,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

fn has_read_access(
    org: OrganizationId,
    proj: Option<ProjectId>,
    stream: Option<StreamId>,
    message: Option<MessageId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, proj, stream, message, user)
}

fn has_write_access(
    org: OrganizationId,
    _proj: Option<ProjectId>,
    _stream: Option<StreamId>,
    _message: Option<MessageId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
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
) -> ApiResult<Vec<Message>> {
    has_read_access(org_id, project_id, stream_id, None, &user)?;

    let messages = repo
        .list_message_metadata(org_id, project_id, stream_id, filter)
        .await?;

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
) -> ApiResult<Message> {
    has_read_access(org_id, project_id, stream_id, Some(message_id), &user)?;

    let message = repo
        .find_by_id(org_id, project_id, stream_id, message_id)
        .await?;
    Ok(Json(message))
}
