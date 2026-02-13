use crate::{
    api::{
        ApiState,
        auth::Authenticated,
        error::{ApiResult, AppError},
    },
    models::{
        NewProject, OrganizationId, OrganizationRepository, Project, ProjectId, ProjectRepository,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;
use tracing::{debug, info};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_projects, create_project,))
        .routes(routes!(update_project, remove_project))
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(test, derive(Serialize))]
pub struct ApiNewProject {
    pub name: String,
    pub retention_period_days: Option<i32>,
}

impl ApiNewProject {
    pub fn fill_defaults(self, max_retention_period: i32) -> NewProject {
        NewProject {
            name: self.name,
            retention_period_days: self.retention_period_days.unwrap_or(max_retention_period),
        }
    }
}

/// List projects
///
/// List all projects under the requested organization the authenticated user has access to
#[utoipa::path(get, path = "/organizations/{org_id}/projects",
    tags = ["Projects"],
    responses(
        (status = 200, description = "Successfully fetched projects", body = [Project]),
        AppError,
    )
)]
pub async fn list_projects(
    State(repo): State<ProjectRepository>,
    Path((org_id,)): Path<(OrganizationId,)>,
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
    tags = ["Projects"],
    request_body = ApiNewProject,
    responses(
        (status = 200, description = "Project successfully updated", body = Project),
        AppError,
    )
)]
pub async fn update_project(
    State(repo): State<ProjectRepository>,
    State(org_repo): State<OrganizationRepository>,
    Path((org_id, proj_id)): Path<(OrganizationId, ProjectId)>,
    user: Box<dyn Authenticated>,
    Json(update): Json<NewProject>,
) -> ApiResult<Project> {
    user.has_org_write_access(&org_id)?;

    let max_retention = org_repo.max_retention_period(org_id).await?;
    if update.retention_period_days < 1 || update.retention_period_days > max_retention {
        return Err(AppError::BadRequest(format!(
            "Retention period must be between 1 and {max_retention}."
        )));
    }

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
    tags = ["Projects"],
    request_body = ApiNewProject,
    responses(
        (status = 201, description = "Project created successfully", body = Project),
        AppError,
    )
)]
pub async fn create_project(
    State(repo): State<ProjectRepository>,
    State(org_repo): State<OrganizationRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id,)): Path<(OrganizationId,)>,
    Json(new): Json<ApiNewProject>,
) -> Result<impl IntoResponse, AppError> {
    user.has_org_write_access(&org_id)?;

    if !org_repo.can_create_new_project(org_id).await? {
        return Err(AppError::Conflict(
            "Organization is not allowed to create more projects, upgrade your subscription to increase the limit.".to_owned(),
        ));
    }

    let max_retention = org_repo.max_retention_period(org_id).await?;
    let new = new.fill_defaults(max_retention);

    if new.retention_period_days < 1 || new.retention_period_days > max_retention {
        return Err(AppError::BadRequest(format!(
            "Retention period must be between 1 and {max_retention}."
        )));
    }

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
    tags = ["Projects"],
    responses(
        (status = 200, description = "Project successfully deleted", body = ProjectId),
        AppError,
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
    use chrono::Utc;
    use sqlx::PgPool;

    use crate::{
        ProductIdentifier, SubscriptionStatus,
        api::tests::{TestServer, deserialize_body, serialize_body},
        mock_subscription,
    };

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
                serialize_body(&ApiNewProject {
                    name: "Test Project".to_string(),
                    retention_period_days: None, // use default retention
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let project: Project = deserialize_body(response.into_body()).await;
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.retention_period_days, 1); // organization has the free tier subscription

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
                    retention_period_days: 1,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let project: Project = deserialize_body(response.into_body()).await;
        assert_eq!(project.name, "Updated Project");
        assert_eq!(project.retention_period_days, 1);

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
                serialize_body(&ApiNewProject {
                    name: "Test Project".to_string(),
                    retention_period_days: Some(1),
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
                    retention_period_days: 1,
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

    async fn set_subscription(pool: &PgPool, org_id: OrganizationId, sub: SubscriptionStatus) {
        sqlx::query!(
            r#"
            UPDATE organizations
            SET current_subscription = $2
            WHERE id = $1
            "#,
            *org_id,
            serde_json::to_value(&sub).unwrap()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_project_creation_limit(pool: PgPool) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let server = TestServer::new(pool.clone(), Some(user_a)).await;

        // cannot create a project without subscription
        set_subscription(&pool, org_1, SubscriptionStatus::None).await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project".to_string(),
                    retention_period_days: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // cannot create a project without subscription
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Active(mock_subscription(ProductIdentifier::NotSubscribed, None)),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project".to_string(),
                    retention_period_days: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // cannot create a project with an expired subscription
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Expired(mock_subscription(
                ProductIdentifier::RmlsLargeMonthly,
                Utc::now().date_naive(),
            )),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project".to_string(),
                    retention_period_days: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // can create 1 project with a free subscription
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Active(mock_subscription(ProductIdentifier::RmlsFree, None)),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 1".to_string(),
                    retention_period_days: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // creating a second project with a free subscription fails
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 2".to_string(),
                    retention_period_days: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // other subscriptions can create multiple projects
        for (i, product) in [
            ProductIdentifier::RmlsTinyMonthly,
            ProductIdentifier::RmlsSmallMonthly,
            ProductIdentifier::RmlsMediumMonthly,
            ProductIdentifier::RmlsLargeMonthly,
            ProductIdentifier::RmlsTinyYearly,
            ProductIdentifier::RmlsSmallYearly,
            ProductIdentifier::RmlsMediumYearly,
            ProductIdentifier::RmlsLargeYearly,
        ]
        .into_iter()
        .enumerate()
        {
            set_subscription(
                &pool,
                org_1,
                SubscriptionStatus::Active(mock_subscription(product, None)),
            )
            .await;
            let response = server
                .post(
                    format!("/api/organizations/{org_1}/projects"),
                    serialize_body(&ApiNewProject {
                        name: format!("Test Project {}", i + 2),
                        retention_period_days: Some(3), // all paid subscriptions allow at least 3 day retention
                    }),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_project_retention_limit(pool: PgPool) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let server = TestServer::new(pool.clone(), Some(user_a)).await;

        // cannot create a project with a longer retention period with a free subscription
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Active(mock_subscription(ProductIdentifier::RmlsFree, None)),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 1".to_string(),
                    retention_period_days: Some(3),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // tiny subscription cannot use longest retention period
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Active(mock_subscription(ProductIdentifier::RmlsTinyMonthly, None)),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 1".to_string(),
                    retention_period_days: Some(30),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // large subscription can use longest retention period
        set_subscription(
            &pool,
            org_1,
            SubscriptionStatus::Active(mock_subscription(
                ProductIdentifier::RmlsLargeMonthly,
                None,
            )),
        )
        .await;
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 1".to_string(),
                    retention_period_days: Some(30),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let proj: Project = deserialize_body(response.into_body()).await;
        assert_eq!(proj.retention_period_days, 30);
        let proj_id = proj.id();

        // large subscription can't create project with more than 30 day retention
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects"),
                serialize_body(&ApiNewProject {
                    name: "Test Project 1".to_string(),
                    retention_period_days: Some(31),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // large subscription can't update project to more than 30 day retention
        let response = server
            .put(
                format!("/api/organizations/{org_1}/projects/{proj_id}"),
                serialize_body(&NewProject {
                    name: "Updated Project".to_string(),
                    retention_period_days: 31,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // large subscription can update project to lower retention
        let response = server
            .put(
                format!("/api/organizations/{org_1}/projects/{proj_id}"),
                serialize_body(&ApiNewProject {
                    name: "Updated Project".to_string(),
                    retention_period_days: Some(7),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let proj: Project = deserialize_body(response.into_body()).await;
        assert_eq!(proj.retention_period_days, 7);
    }
}
