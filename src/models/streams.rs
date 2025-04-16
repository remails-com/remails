use crate::models::{Error, OrganizationId, ProjectId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, sqlx::Type, FromStr,
)]
#[sqlx(transparent)]
pub struct StreamId(Uuid);

impl StreamId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Serialize)]
pub struct Stream {
    id: StreamId,
    project_id: ProjectId,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NewStream {
    name: String,
}

pub struct StreamRepository {
    pool: PgPool,
}

impl StreamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        new: NewStream,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> Result<Stream, Error> {
        let correct_org = sqlx::query_scalar!(
            r#"
            SELECT id FROM projects WHERE id = $1
            "#,
            *project_id
        )
        .fetch_one(&self.pool)
        .await?;

        if correct_org != *organization_id {
            Err(Error::BadRequest(
                "The provided organization and project IDs do not match".to_string(),
            ))?
        }

        let project = sqlx::query_as!(
            Stream,
            r#"
            INSERT INTO streams (id, project_id, name) 
            VALUES (gen_random_uuid(), $1, $2)
            RETURNING *
            "#,
            *project_id,
            new.name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(project)
    }

    pub async fn list(
        &self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> Result<Vec<Stream>, Error> {
        Ok(sqlx::query_as!(
            Stream,
            r#"
            SELECT s.id,
                   s.project_id,
                   s.name,
                   s.created_at,
                   s.updated_at
            FROM streams s
                JOIN projects p ON s.project_id = p.id 
                     WHERE s.project_id = $1 
                       AND p.organization_id = $2 
            "#,
            *project_id,
            *organization_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        organization_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
    ) -> Result<StreamId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM streams s 
                   USING projects p 
                   WHERE s.project_id = p.id 
                     AND s.id = $1
                     AND s.project_id = $2
                     AND p.organization_id = $3
            RETURNING s.id
            "#,
            *stream_id,
            *project_id,
            *organization_id
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }
}
