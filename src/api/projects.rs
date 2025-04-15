use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiUser, NewProject, OrganizationId, Project, ProjectId, ProjectRepository},
};
use axum::{
    Json,
    extract::{Path, State},
};

fn has_read_access(org: OrganizationId, user: &ApiUser) -> Result<(), ApiError> {
    has_write_access(org, user)
}

fn has_write_access(org: OrganizationId, user: &ApiUser) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn list_projects(
    State(repo): State<ProjectRepository>,
    Path(org): Path<OrganizationId>,
    api_user: ApiUser,
) -> ApiResult<Vec<Project>> {
    has_read_access(org, &api_user)?;

    Ok(Json(repo.list(org).await?))
}

pub async fn create_project(
    State(repo): State<ProjectRepository>,
    api_user: ApiUser,
    Path(org): Path<OrganizationId>,
    Json(new): Json<NewProject>,
) -> ApiResult<Project> {
    has_write_access(org, &api_user)?;

    Ok(Json(repo.create(new, org).await?))
}

pub async fn remove_project(
    State(repo): State<ProjectRepository>,
    api_user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
) -> Result<(), ApiError> {
    has_write_access(org, &api_user)?;

    repo.remove(proj, org).await?;
    Ok(())
}
