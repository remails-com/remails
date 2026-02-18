use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use garde::Validate;
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
    pub retention_period_days: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Project {
    pub fn id(&self) -> ProjectId {
        self.id
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[cfg_attr(test, derive(Serialize))]
pub struct NewProject {
    #[schema(min_length = 1, max_length = 256)]
    #[garde(length(min = 1, max = 256))]
    pub name: String,
    /// Set the retention period for emails within this project in days.
    ///
    /// This must be a value between 1 and the maximum retention period for your subscription.
    #[schema(minimum = 1, maximum = 30)]
    #[garde(range(min = 1, max = 30))]
    pub retention_period_days: i32,
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
        if new.retention_period_days < 1 || new.retention_period_days > 30 {
            return Err(Error::Internal(format!(
                "Invalid retention period ({})",
                new.retention_period_days
            )));
        }

        Ok(sqlx::query_as!(
            Project,
            r#"
            INSERT INTO projects (id, organization_id, name, retention_period_days)
            VALUES (gen_random_uuid(), $1, $2, $3)
            RETURNING *
            "#,
            *organization_id,
            new.name.trim(),
            new.retention_period_days,
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
        if update.retention_period_days < 1 || update.retention_period_days > 30 {
            return Err(Error::Internal(format!(
                "Invalid retention period ({})",
                update.retention_period_days
            )));
        }

        Ok(sqlx::query_as!(
            Project,
            r#"
            UPDATE projects 
            SET name = $3,
                retention_period_days = $4
            WHERE id = $2
              AND organization_id = $1
            RETURNING *
            "#,
            *organization_id,
            *project_id,
            update.name.trim(),
            update.retention_period_days,
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

#[cfg(test)]
mod test {
    use crate::test::TestProjects;

    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn project_lifecycle(db: PgPool) {
        let org_1 = TestProjects::Org1Project1.org_id();
        let repo = ProjectRepository::new(db);

        // no projects
        assert_eq!(repo.list(org_1).await.unwrap().len(), 0);

        // create project
        let project = repo
            .create(
                NewProject {
                    name: "New Project".to_owned(),
                    retention_period_days: 1,
                },
                org_1,
            )
            .await
            .unwrap();
        assert_eq!(project.name, "New Project");
        assert_eq!(project.retention_period_days, 1);
        assert_eq!(project.organization_id, org_1);

        // list projects
        let projects = repo.list(org_1).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id(), project.id());

        // update project
        let project = repo
            .update(
                org_1,
                project.id(),
                NewProject {
                    name: "Updated Project".to_owned(),
                    retention_period_days: 3,
                },
            )
            .await
            .unwrap();
        assert_eq!(project.name, "Updated Project");
        assert_eq!(project.retention_period_days, 3);
        assert_eq!(project.organization_id, org_1);
        assert_eq!(projects[0].id(), project.id());

        // remove project
        assert_eq!(
            repo.remove(project.id(), org_1).await.unwrap(),
            project.id()
        );

        // no projects
        assert_eq!(repo.list(org_1).await.unwrap().len(), 0);
    }

    /// Test that retention period is limited to a reasonable amount
    ///
    /// Note that this does not enforce the subscription-based retention limits,
    /// these are enforced within the API layer
    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn retention_period_limit(db: PgPool) {
        let org_1 = TestProjects::Org1Project1.org_id();
        let repo = ProjectRepository::new(db);

        let mut n = 0;
        let mut new_project = |retention_period_days| {
            n += 1;
            NewProject {
                name: format!("Project {n}"),
                retention_period_days,
            }
        };

        let project = repo.create(new_project(1), org_1).await.unwrap();
        let id = project.id();

        // 30 days is the maximum allowed retention period
        repo.create(new_project(30), org_1).await.unwrap();
        repo.update(org_1, id, new_project(30)).await.unwrap();

        // >30 days is not allowed because it could cause issues with the statistics tracking message clean up system
        repo.create(new_project(31), org_1).await.unwrap_err();
        repo.update(org_1, id, new_project(31)).await.unwrap_err();

        // 0 days is not allowed because it would risk deleting messages that haven't been attempted to send yet
        repo.create(new_project(0), org_1).await.unwrap_err();
        repo.update(org_1, id, new_project(0)).await.unwrap_err();
    }
}
