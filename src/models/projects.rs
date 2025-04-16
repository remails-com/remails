use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
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
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

    pub async fn remove(&self, id: ProjectId, org_id: OrganizationId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            DELETE FROM projects
                   WHERE id = $1
                     AND organization_id = $2
            "#,
            *id,
            *org_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
