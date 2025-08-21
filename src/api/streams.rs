use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiUser, NewStream, OrganizationId, ProjectId, Stream, StreamId, StreamRepository},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

fn has_read_access(
    org: &OrganizationId,
    proj: &ProjectId,
    stream: Option<&StreamId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, proj, stream, user)
}

fn has_write_access(
    org: &OrganizationId,
    _proj: &ProjectId,
    _stream: Option<&StreamId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.is_org_admin(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn list_streams(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
) -> ApiResult<Vec<Stream>> {
    has_read_access(&org, &proj, None, &user)?;

    let streams = repo.list(org, proj).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = proj.to_string(),
        "listed {} streams",
        streams.len()
    );

    Ok(Json(streams))
}

pub async fn create_stream(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
    Json(new): Json<NewStream>,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(&org, &proj, None, &user)?;

    let stream = repo.create(new, org, proj).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = proj.to_string(),
        stream_id = stream.id().to_string(),
        stream_name = stream.name,
        "created stream"
    );

    Ok((StatusCode::CREATED, Json(stream)))
}

pub async fn update_stream(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj, stream)): Path<(OrganizationId, ProjectId, StreamId)>,
    Json(update): Json<NewStream>,
) -> ApiResult<Stream> {
    has_write_access(&org, &proj, Some(&stream), &user)?;

    let stream = repo.update(org, proj, stream, update).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = proj.to_string(),
        stream_id = stream.id().to_string(),
        stream_name = stream.name,
        "updated stream"
    );

    Ok(Json(stream))
}

pub async fn remove_stream(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj, stream)): Path<(OrganizationId, ProjectId, StreamId)>,
) -> ApiResult<StreamId> {
    has_write_access(&org, &proj, Some(&stream), &user)?;

    let stream_id = repo.remove(org, proj, stream).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = proj.to_string(),
        stream_id = stream_id.to_string(),
        "deleted stream",
    );

    Ok(Json(stream_id))
}
