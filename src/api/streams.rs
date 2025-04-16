use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiUser, NewStream, OrganizationId, ProjectId, Stream, StreamId, StreamRepository},
};
use axum::{
    Json,
    extract::{Path, State},
};

fn has_read_access(org: OrganizationId, proj: ProjectId, user: &ApiUser) -> Result<(), ApiError> {
    has_write_access(org, proj, user)
}

fn has_write_access(org: OrganizationId, _proj: ProjectId, user: &ApiUser) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn list_streams(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
) -> ApiResult<Vec<Stream>> {
    has_read_access(org, proj, &user)?;

    Ok(Json(repo.list(org, proj).await?))
}

pub async fn create_stream(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
    Json(new): Json<NewStream>,
) -> ApiResult<Stream> {
    has_write_access(org, proj, &user)?;

    Ok(Json(repo.create(new, org, proj).await?))
}

pub async fn remove_stream(
    State(repo): State<StreamRepository>,
    user: ApiUser,
    Path((org, proj, stream)): Path<(OrganizationId, ProjectId, StreamId)>,
) -> ApiResult<StreamId> {
    has_write_access(org, proj, &user)?;

    Ok(Json(repo.remove(org, proj, stream).await?))
}
