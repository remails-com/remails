use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiUser, NewProject, OrganizationId, Project, ProjectId, ProjectRepository},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

fn has_read_access(
    org: OrganizationId,
    project: Option<ProjectId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, project, user)
}

fn has_write_access(
    org: OrganizationId,
    _project: Option<ProjectId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn list_projects(
    State(repo): State<ProjectRepository>,
    Path(org): Path<OrganizationId>,
    user: ApiUser,
) -> ApiResult<Vec<Project>> {
    has_read_access(org, None, &user)?;

    let projects = repo.list(org).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        "listed {} projects",
        projects.len()
    );

    Ok(Json(projects))
}

pub async fn update_project(
    State(repo): State<ProjectRepository>,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
    user: ApiUser,
    Json(update): Json<NewProject>,
) -> ApiResult<Project> {
    has_write_access(org, Some(proj), &user)?;

    let project = repo.update(org, proj, update).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = proj.to_string(),
        "updated project",
    );

    Ok(Json(project))
}

pub async fn create_project(
    State(repo): State<ProjectRepository>,
    user: ApiUser,
    Path(org): Path<OrganizationId>,
    Json(new): Json<NewProject>,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(org, None, &user)?;

    let project = repo.create(new, org).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = project.id().to_string(),
        project_name = project.name,
        "created project"
    );

    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn remove_project(
    State(repo): State<ProjectRepository>,
    user: ApiUser,
    Path((org, proj)): Path<(OrganizationId, ProjectId)>,
) -> ApiResult<ProjectId> {
    has_write_access(org, Some(proj), &user)?;

    let project_id = repo.remove(proj, org).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        project_id = project_id.to_string(),
        "deleted project",
    );

    Ok(Json(project_id))
}
