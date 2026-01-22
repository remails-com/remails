use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    sqlx::Type,
    FromStr,
    Eq,
    IntoParams,
    ToSchema,
)]
#[sqlx(transparent)]
#[into_params(names("proj_id"))]
pub struct ProjectId(Uuid);

impl ProjectId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct Project {
    id: ProjectId,
    organization_id: OrganizationId,
    pub name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    retention_period_days: i32,
}

impl Project {
    pub fn id(&self) -> ProjectId {
        self.id
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(test, derive(Serialize))]
pub struct NewProject {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ProjectRepository {
    pool: sqlx::PgPool,
}

impl ProjectRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        new: NewProject,
        organization_id: OrganizationId,
    ) -> Result<Project, Error> {
        Ok(sqlx::query_as!(
            Project,
            r#"
            INSERT INTO projects (id, organization_id, name)
            VALUES (gen_random_uuid(), $1, $2)
            RETURNING *
            "#,
            *organization_id,
            new.name.trim(),
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list(&self, organization_id: OrganizationId) -> Result<Vec<Project>, Error> {
        Ok(sqlx::query_as!(
            Project,
            r#"
            SELECT * FROM projects WHERE organization_id = $1 ORDER BY updated_at DESC
            "#,
            *organization_id,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn update(
        &self,
        organization_id: OrganizationId,
        project_id: ProjectId,
        update: NewProject,
    ) -> Result<Project, Error> {
        Ok(sqlx::query_as!(
            Project,
            r#"
            UPDATE projects 
            SET name = $3 
            WHERE id = $2
              AND organization_id = $1
            RETURNING *
            "#,
            *organization_id,
            *project_id,
            update.name.trim(),
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn remove(&self, id: ProjectId, org_id: OrganizationId) -> Result<ProjectId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM projects
                   WHERE id = $1
                     AND organization_id = $2
            RETURNING id
            "#,
            *id,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }
}
