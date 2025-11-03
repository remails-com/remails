use crate::models::{Error, OrganizationId, ProjectId, projects};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use garde::Validate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::debug;
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
    IntoParams,
    ToSchema,
)]
#[sqlx(transparent)]
#[into_params(names("stream_id"))]
pub struct StreamId(Uuid);

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct Stream {
    id: StreamId,
    project_id: ProjectId,
    pub name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Stream {
    pub fn id(&self) -> StreamId {
        self.id
    }
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[cfg_attr(test, derive(Serialize))]
pub struct NewStream {
    #[garde(length(min = 1, max = 256))]
    #[schema(min_length = 1, max_length = 256)]
    pub name: String,
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
            SELECT organization_id FROM projects WHERE id = $1
            "#,
            *project_id
        )
        .fetch_one(&self.pool)
        .await?;

        if correct_org != *organization_id {
            debug!(
                correct_org = correct_org.to_string(),
                organization_id = organization_id.to_string(),
                "The provided organization and project IDs do not match"
            );
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
            new.name.trim()
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
        if !projects::check_org_match(organization_id, project_id, &self.pool).await? {
            return Err(Error::BadRequest(
                "Project ID does not match organization ID".to_string(),
            ));
        }

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

    pub async fn update(
        &self,
        organization_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        update: NewStream,
    ) -> Result<Stream, Error> {
        Ok(sqlx::query_as!(
            Stream,
            r#"
            UPDATE streams s
                   SET name = $4
            FROM projects p
            WHERE s.id = $3
              AND s.project_id = $2
              AND s.project_id = p.id
              AND p.organization_id = $1
            returning s.*
            "#,
            *organization_id,
            *project_id,
            *stream_id,
            update.name.trim()
        )
        .fetch_one(&self.pool)
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

#[cfg(test)]
mod tests {
    use crate::models::{Error, NewStream};
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects")))]
    async fn create_happy_flow(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let stream = repo
            .create(
                NewStream {
                    name: "test Stream".to_string(),
                },
                // Organization 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(stream.name, "test Stream");
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects")))]
    async fn create_org_does_not_match_proj(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let bad_request = repo
            .create(
                NewStream {
                    name: "test Stream".to_string(),
                },
                // Organization 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(bad_request, Error::BadRequest(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn update_happy_flow(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let updated = repo
            .update(
                // Organization 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
                NewStream {
                    name: "Updated Stream Name".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated Stream Name");
        assert_eq!(
            updated.id,
            "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap()
        );
        assert_ne!(updated.created_at, updated.updated_at);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn update_organization_does_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .update(
                // Organization 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
                NewStream {
                    name: "Updated Stream Name".to_string(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn update_project_does_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .update(
                // Organization 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 2 Organization 1
                "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
                NewStream {
                    name: "Updated Stream Name".to_string(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn update_project_and_org_do_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .update(
                // Organization 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 2
                "70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
                NewStream {
                    name: "Updated Stream Name".to_string(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn remove_happy_flow(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let id = repo
            .remove(
                // Organization 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(id, "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),)
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn remove_organization_does_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .remove(
                // Organization 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 1
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn remove_project_does_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .remove(
                // Organization 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 2 Organization 1
                "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn remove_project_and_org_do_not_match(db: PgPool) {
        let repo = super::StreamRepository::new(db);

        let err = repo
            .remove(
                // Organization 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 2
                "70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap(),
                // Stream 1 Project 1 Organization 1
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }
}
