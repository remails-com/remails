use crate::models::{Actor, AuditLogRepository, Error, OrganizationId};
use chrono::{DateTime, Utc};
use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

id!(
    #[derive(IntoParams)]
    #[into_params(names("proj_id"))]
    ProjectId
);

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
    pub plaintext_fallback: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Project {
    pub fn id(&self) -> ProjectId {
        self.id
    }

    pub fn org_id(&self) -> OrganizationId {
        self.organization_id
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
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
    /// If set true, emails in the project will fall back to being sent without TLS encryption
    /// if delivery over TLS fails.
    #[garde(skip)]
    pub plaintext_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectRepository {
    pool: sqlx::PgPool,
    audit_log: AuditLogRepository,
}

impl ProjectRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            audit_log: AuditLogRepository::new(pool.clone()),
            pool,
        }
    }

    pub async fn create(
        &self,
        new: &NewProject,
        organization_id: OrganizationId,
        actor: impl Into<Actor>,
    ) -> Result<Project, Error> {
        if new.retention_period_days < 1 || new.retention_period_days > 30 {
            return Err(Error::Internal(format!(
                "Invalid retention period ({})",
                new.retention_period_days
            )));
        }

        let mut tx = self.pool.begin().await?;
        let project = sqlx::query_as!(
            Project,
            r#"
            INSERT INTO projects (id, organization_id, name, retention_period_days, plaintext_fallback)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING *
            "#,
            *organization_id,
            new.name.trim(),
            new.retention_period_days,
            new.plaintext_fallback
        )
        .fetch_one(&mut *tx)
        .await?;

        self.audit_log
            .log(
                &mut tx,
                actor,
                &project,
                "Created project",
                Some(json!(new)),
            )
            .await?;

        tx.commit().await?;
        Ok(project)
    }

    pub async fn get(&self, project_id: ProjectId) -> Result<Project, Error> {
        Ok(sqlx::query_as!(
            Project,
            r#"
            SELECT * FROM projects WHERE id = $1
            "#,
            *project_id,
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
        update: &NewProject,
        actor: impl Into<Actor>,
    ) -> Result<Project, Error> {
        if update.retention_period_days < 1 || update.retention_period_days > 30 {
            return Err(Error::Internal(format!(
                "Invalid retention period ({})",
                update.retention_period_days
            )));
        }

        let mut tx = self.pool.begin().await?;
        let project = sqlx::query_as!(
            Project,
            r#"
            UPDATE projects 
            SET name = $3,
                retention_period_days = $4,
                plaintext_fallback = $5
            WHERE id = $2
              AND organization_id = $1
            RETURNING *
            "#,
            *organization_id,
            *project_id,
            update.name.trim(),
            update.retention_period_days,
            update.plaintext_fallback,
        )
        .fetch_one(&mut *tx)
        .await?;

        self.audit_log
            .log(
                &mut tx,
                actor,
                &project,
                "Updated project",
                Some(json!(update)),
            )
            .await?;

        tx.commit().await?;
        Ok(project)
    }

    pub async fn remove(
        &self,
        id: ProjectId,
        org_id: OrganizationId,
        actor: impl Into<Actor>,
    ) -> Result<ProjectId, Error> {
        let mut tx = self.pool.begin().await?;
        let removed_id: ProjectId = sqlx::query_scalar!(
            r#"
            DELETE FROM projects
                   WHERE id = $1
                     AND organization_id = $2
            RETURNING id
            "#,
            *id,
            *org_id
        )
        .fetch_one(&mut *tx)
        .await?
        .into();

        self.audit_log
            .log(
                &mut tx,
                actor,
                (removed_id, org_id),
                "Deleted project",
                None,
            )
            .await?;

        tx.commit().await?;
        Ok(removed_id)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        models::{AuditLogRepository, SYSTEM},
        test::TestProjects,
    };

    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn project_lifecycle(db: PgPool) {
        let org_1 = TestProjects::Org1Project1.org_id();
        let repo = ProjectRepository::new(db.clone());
        let audit_log = AuditLogRepository::new(db);

        // no projects
        assert_eq!(repo.list(org_1).await.unwrap().len(), 0);

        // create project
        let project = repo
            .create(
                &NewProject {
                    name: "New Project".to_owned(),
                    retention_period_days: 1,
                    plaintext_fallback: false,
                },
                org_1,
                SYSTEM,
            )
            .await
            .unwrap();
        assert_eq!(project.name, "New Project");
        assert_eq!(project.retention_period_days, 1);
        assert_eq!(project.organization_id, org_1);
        assert!(!project.plaintext_fallback);
        let audit_entries = audit_log.list(org_1).await.unwrap();
        assert_eq!(audit_entries.len(), 1);
        assert_eq!(audit_entries[0].target_id, Some(*project.id()));
        assert_eq!(audit_entries[0].action, "Created project");

        // get project
        let proj = repo.get(project.id).await.unwrap();
        assert_eq!(proj.name, project.name);
        assert_eq!(proj.retention_period_days, project.retention_period_days);
        assert_eq!(proj.organization_id, project.organization_id);
        assert_eq!(proj.plaintext_fallback, project.plaintext_fallback);

        // list projects
        let projects = repo.list(org_1).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id(), project.id());

        // update project
        let project = repo
            .update(
                org_1,
                project.id(),
                &NewProject {
                    name: "Updated Project".to_owned(),
                    retention_period_days: 3,
                    plaintext_fallback: false,
                },
                SYSTEM,
            )
            .await
            .unwrap();
        assert_eq!(project.name, "Updated Project");
        assert_eq!(project.retention_period_days, 3);
        assert_eq!(project.organization_id, org_1);
        assert_eq!(projects[0].id(), project.id());
        let audit_entries = audit_log.list(org_1).await.unwrap();
        assert_eq!(audit_entries.len(), 2);
        assert_eq!(audit_entries[0].target_id, Some(*project.id()));
        assert_eq!(audit_entries[0].action, "Updated project");

        // remove project
        assert_eq!(
            repo.remove(project.id(), org_1, SYSTEM).await.unwrap(),
            project.id()
        );
        let audit_entries = audit_log.list(org_1).await.unwrap();
        assert_eq!(audit_entries.len(), 3);
        assert_eq!(audit_entries[0].target_id, Some(*project.id()));
        assert_eq!(audit_entries[0].action, "Deleted project");

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
                plaintext_fallback: false,
            }
        };

        let project = repo.create(&new_project(1), org_1, SYSTEM).await.unwrap();
        let id = project.id();

        // 30 days is the maximum allowed retention period
        repo.create(&new_project(30), org_1, SYSTEM).await.unwrap();
        repo.update(org_1, id, &new_project(30), SYSTEM)
            .await
            .unwrap();

        // >30 days is not allowed because it could cause issues with the statistics tracking message clean up system
        repo.create(&new_project(31), org_1, SYSTEM)
            .await
            .unwrap_err();
        repo.update(org_1, id, &new_project(31), SYSTEM)
            .await
            .unwrap_err();

        // 0 days is not allowed because it would risk deleting messages that haven't been attempted to send yet
        repo.create(&new_project(0), org_1, SYSTEM)
            .await
            .unwrap_err();
        repo.update(org_1, id, &new_project(0), SYSTEM)
            .await
            .unwrap_err();
    }
}
