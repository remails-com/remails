use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Postgres};
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
)]
#[sqlx(transparent)]
pub struct ProjectId(Uuid);

impl ProjectId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Serialize)]
pub struct Project {
    id: ProjectId,
    organization_id: OrganizationId,
    pub name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Project {
    pub fn id(&self) -> ProjectId {
        self.id
    }
}

#[derive(Debug, Deserialize)]
pub struct NewProject {
    name: String,
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
            new.name,
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list(&self, organization_id: OrganizationId) -> Result<Vec<Project>, Error> {
        Ok(sqlx::query_as!(
            Project,
            r#"
            SELECT * FROM projects WHERE organization_id = $1
            "#,
            *organization_id,
        )
        .fetch_all(&self.pool)
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

pub async fn check_org_match<'e, E>(
    org_id: OrganizationId,
    proj_id: ProjectId,
    executor: E,
) -> Result<bool, Error>
where
    E: 'e + Executor<'e, Database = Postgres>,
{
    let pg_org_id = sqlx::query_scalar!(
        r#"
        SELECT organization_id FROM projects WHERE id = $1
        "#,
        *proj_id
    )
    .fetch_one(executor)
    .await?;

    Ok(pg_org_id == *org_id)
}
