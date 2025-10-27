use crate::{
    api::{
        ApiState,
        auth::Authenticated,
        error::{ApiError, ApiResult},
    },
    models::{NewProject, OrganizationId, Project, ProjectId, ProjectRepository},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_projects, create_project,))
        .routes(routes!(update_project, remove_project))
}

/// List projects
///
/// List all projects under the requested organization the authenticated user has access to
#[utoipa::path(get, path = "/organizations/{org_id}/projects",
    params(
        OrganizationId
    ),
    responses(
        (status = 200, description = "Successfully fetched projects", body = [Project]),
        ApiError,
    )
)]
pub async fn list_projects(
    State(repo): State<ProjectRepository>,
    Path(org_id): Path<OrganizationId>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Vec<Project>> {
    user.has_org_read_access(&org_id)?;

    let projects = repo.list(org_id).await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        "listed {} projects",
        projects.len()
    );

    Ok(Json(projects))
}

/// Update a project
///
/// Update details about that project
#[utoipa::path(put, path = "/organizations/{org_id}/projects/{proj_id}",
    params(
        OrganizationId, ProjectId
    ),
    request_body = NewProject,
    responses(
        (status = 200, description = "Project successfully updated", body = Project),
        ApiError,
    )
)]
pub async fn update_project(
    State(repo): State<ProjectRepository>,
    Path((org_id, proj_id)): Path<(OrganizationId, ProjectId)>,
    user: Box<dyn Authenticated>,
    Json(update): Json<NewProject>,
) -> ApiResult<Project> {
    user.has_org_write_access(&org_id)?;

    let project = repo.update(org_id, proj_id, update).await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        "updated project",
    );

    Ok(Json(project))
}

/// Create a new project
///
/// Create a new project under the specified organization
#[utoipa::path(post, path = "/organizations/{org_id}/projects",
    params(
        OrganizationId
    ),
    request_body = NewProject,
    responses(
        (status = 201, description = "Project created successfully", body = Project),
        ApiError,
    )
)]
pub async fn create_project(
    State(repo): State<ProjectRepository>,
    user: Box<dyn Authenticated>,
    Path(org_id): Path<OrganizationId>,
    Json(new): Json<NewProject>,
) -> Result<impl IntoResponse, ApiError> {
    user.has_org_write_access(&org_id)?;

    let project = repo.create(new, org_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project.id().to_string(),
        project_name = project.name,
        "created project"
    );

    Ok((StatusCode::CREATED, Json(project)))
}

/// Delete a project
#[utoipa::path(delete, path = "/organizations/{org_id}/projects/{proj_id}",
    params(
        OrganizationId, ProjectId
    ),
    responses(
        (status = 200, description = "Project successfully deleted", body = ProjectId),
        ApiError,
    )
)]
pub async fn remove_project(
    State(repo): State<ProjectRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id)): Path<(OrganizationId, ProjectId)>,
) -> ApiResult<ProjectId> {
    user.has_org_write_access(&org_id)?;

    let project_id = repo.remove(proj_id, org_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.to_string(),
        "deleted project",
    );

    Ok(Json(project_id))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::api::tests::{TestServer, deserialize_body, serialize_body};

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_project_lifecycle(pool: PgPool) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let server = TestServer::new(pool.clone(), Some(user_a)).await;

        // start with no projects
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let projects: Vec<Project> = deserialize_body(response.into_body()).await;
        assert_eq!(projects.len(), 0);

        // create a project
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&NewProject {
                    name: "Test Project".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let project: Project = deserialize_body(response.into_body()).await;
        assert_eq!(project.name, "Test Project");

        // list projects
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let projects: Vec<Project> = deserialize_body(response.into_body()).await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Test Project");

        // update project
        let response = server
            .put(
                format!("/api/organizations/{org_1}/projects/{}", project.id()),
                serialize_body(&NewProject {
                    name: "Updated Project".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let project: Project = deserialize_body(response.into_body()).await;
        assert_eq!(project.name, "Updated Project");

        // list projects
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let projects: Vec<Project> = deserialize_body(response.into_body()).await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Updated Project");

        // remove project
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{}",
                project.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // list projects
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let projects: Vec<Project> = deserialize_body(response.into_body()).await;
        assert_eq!(projects.len(), 0);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn test_project_no_access(pool: PgPool) {
        let user_b = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462";
        let server = TestServer::new(pool.clone(), Some(user_b)).await;

        // can't list projects
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't create projects
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&NewProject {
                    name: "Test Project".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't update projects
        let response = server
            .put(
                format!("/api/organizations/{org_1}/projects/{proj_1}"),
                serialize_body(&NewProject {
                    name: "Updated Project".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't remove projects
        let response = server
            .delete(format!("/api/organizations/{org_1}/projects/{proj_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
