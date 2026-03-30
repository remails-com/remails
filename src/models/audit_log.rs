use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{ApiKey, ApiUser, Error, OrganizationId, Project, ProjectId};

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, sqlx::Type, FromStr,
)]
#[sqlx(transparent)]
pub struct AuditLogId(Uuid);

#[derive(Debug, Display, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "audit_log_target_type", rename_all = "snake_case")]
pub enum TargetType {
    Project,
    Domain,
    Message,
    SmtpCredential,
    ApiKey,
}

#[derive(Debug, Display, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "audit_log_actor_type", rename_all = "snake_case")]
pub enum ActorType {
    ApiUser,
    ApiKey,
    System,
}

#[derive(Serialize)]
pub struct AuditLogEntry {
    id: AuditLogId,
    organization_id: OrganizationId,
    target_id: Uuid,
    target_type: TargetType,
    actor_id: Option<Uuid>,
    actor_type: ActorType,
    action: String,
    details: Option<Value>,
    occurred_at: DateTime<Utc>,
}

pub struct Actor(ActorType, Option<Uuid>);

pub const SYSTEM: Actor = Actor(ActorType::System, None);

impl From<&ApiUser> for Actor {
    fn from(user: &ApiUser) -> Self {
        Actor(ActorType::ApiUser, Some(**user.id()))
    }
}

impl From<&ApiKey> for Actor {
    fn from(user: &ApiKey) -> Self {
        Actor(ActorType::ApiKey, Some(**user.id()))
    }
}

pub struct Target(TargetType, Uuid, OrganizationId);

impl From<&Project> for Target {
    fn from(project: &Project) -> Self {
        Target(TargetType::Project, *project.id(), project.org_id())
    }
}

impl From<(ProjectId, OrganizationId)> for Target {
    fn from((proj_id, org_id): (ProjectId, OrganizationId)) -> Self {
        Target(TargetType::Project, *proj_id, org_id)
    }
}

impl AuditLogEntry {
    pub fn new(actor: Actor, target: Target, action: String, details: Option<Value>) -> Self {
        Self {
            id: AuditLogId(Uuid::new_v4()),
            organization_id: target.2,
            target_id: target.1,
            target_type: target.0,
            actor_id: actor.1,
            actor_type: actor.0,
            action,
            details,
            occurred_at: Utc::now(),
        }
    }
}

// const fn default_limit() -> i64 {
//     10
// }

// #[derive(Debug, Deserialize, IntoParams, Validate)]
// #[serde(default)]
// pub struct AuditLogFilter {
//     #[param(minimum = 1, maximum = 100, default = default_limit)]
//     #[garde(range(min = 1, max = 100))]
//     limit: i64, // TODO: link default
//     #[garde(skip)]
//     actor_id: Option<Vec<Uuid>>,
//     // TODO: add other filters
// }

#[derive(Debug, Clone)]
pub struct AuditLogRepository {
    pool: sqlx::PgPool,
}

impl AuditLogRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn log(
        &self,
        actor: impl Into<Actor>,
        target: impl Into<Target>,
        action: &'static str,
        details: Option<Value>,
    ) -> Result<(), Error> {
        let actor = actor.into();
        let target = target.into();
        tracing::info!(
            actor_type = %actor.0,
            actor_id = actor.1.map(|id| id.to_string()),
            target_type = %target.0,
            target_id = %target.1,
            org_id = %target.2,
            details = details.as_ref().map(|v| v.to_string()),
            "{}",
            action
        );
        self.add(AuditLogEntry::new(
            actor,
            target,
            action.to_owned(),
            details,
        ))
        .await
    }

    async fn add(&self, event: AuditLogEntry) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO audit_log (
                id,
                organization_id,
                target_id,
                target_type,
                actor_id,
                actor_type,
                action,
                details,
                occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            *event.id,
            *event.organization_id,
            event.target_id,
            event.target_type as TargetType,
            event.actor_id,
            event.actor_type as ActorType,
            event.action,
            event.details,
            event.occurred_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list(
        &self,
        org_id: OrganizationId,
        // filter: AuditLogFilter,
    ) -> Result<Vec<AuditLogEntry>, Error> {
        Ok(sqlx::query_as!(
            AuditLogEntry,
            r#"
            SELECT
                id,
                organization_id,
                target_id,
                target_type AS "target_type: TargetType",
                actor_id,
                actor_type AS "actor_type: ActorType",
                action,
                details,
                occurred_at            
            FROM audit_log
            WHERE organization_id = $1
            "#,
            *org_id,
        )
        .fetch_all(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::TestProjects;

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects")))]
    async fn test_audit_log(pool: sqlx::PgPool) {
        let repository = AuditLogRepository::new(pool.clone());

        let org_id = TestProjects::Org1Project1.org_id();
        let project_id: ProjectId = "00000000-0000-4321-0000-000000000000".parse().unwrap();

        repository
            .log(
                SYSTEM,
                (project_id, org_id),
                "test log",
                Some(serde_json::json!({"key": "value"})),
            )
            .await
            .unwrap();

        let logs = repository.list(org_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].target_id, *project_id);
        assert_eq!(logs[0].action, "test log");
        assert_eq!(
            logs[0].details.as_ref().unwrap().get("key").unwrap(),
            "value"
        );
    }
}
