use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::{
    ApiDomain, ApiKey, ApiKeyId, ApiUser, ApiUserId, DomainId, Error, InviteId, MessageId,
    OrganizationId, Project, ProjectId, SmtpCredentialId,
};

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
    ToSchema,
)]
#[sqlx(transparent)]
pub struct AuditLogId(Uuid);

#[derive(
    Debug, Display, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "audit_log_target_type", rename_all = "snake_case")]
pub enum TargetType {
    Project,
    Domain,
    Message,
    SmtpCredential,
    ApiKey,
    InviteLink,
    Member,
}

#[derive(
    Debug, Display, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "audit_log_actor_type", rename_all = "snake_case")]
pub enum ActorType {
    ApiUser,
    ApiKey,
    System,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLogEntry {
    id: AuditLogId,
    organization_id: OrganizationId,
    pub target_id: Option<Uuid>,
    target_type: Option<TargetType>,
    actor_id: Option<Uuid>,
    actor_type: ActorType,
    pub action: String,
    details: Option<Value>,
    occurred_at: DateTime<Utc>,
}

pub struct Actor(ActorType, Option<Uuid>);

#[allow(dead_code)]
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

pub struct Target(Option<TargetType>, Option<Uuid>, OrganizationId);

macro_rules! object_to_target {
    ($obj:ty => $kind:expr) => {
        impl From<&$obj> for Target {
            fn from(object: &$obj) -> Self {
                Target(Some($kind), Some(*object.id()), object.org_id())
            }
        }
    };
}
object_to_target!(Project => TargetType::Project);
object_to_target!(ApiDomain => TargetType::Domain);

macro_rules! id_to_target {
    ($id:ty => $kind:expr) => {
        impl From<($id, OrganizationId)> for Target {
            fn from((id, org_id): ($id, OrganizationId)) -> Self {
                Target(Some($kind), Some(*id), org_id)
            }
        }
    };
}
id_to_target!(ProjectId => TargetType::Project);
id_to_target!(DomainId => TargetType::Domain);
id_to_target!(MessageId => TargetType::Message);
id_to_target!(SmtpCredentialId => TargetType::SmtpCredential);
id_to_target!(ApiKeyId => TargetType::ApiKey);
id_to_target!(InviteId => TargetType::InviteLink);
id_to_target!(ApiUserId => TargetType::Member);

impl From<OrganizationId> for Target {
    fn from(org_id: OrganizationId) -> Self {
        Target(None, None, org_id)
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
        tx: &mut Transaction<'_, Postgres>,
        actor: impl Into<Actor>,
        target: impl Into<Target>,
        action: &'static str,
        details: Option<Value>,
    ) -> Result<(), Error> {
        let actor = actor.into();
        let target = target.into();
        tracing::info!(
            actor_type = actor.0.to_string(),
            actor_id = actor.1.map(|id| id.to_string()),
            target_type = target.0.map(|t| t.to_string()),
            target_id = target.1.map(|id| id.to_string()),
            org_id = target.2.to_string(),
            details = details.as_ref().map(|v| v.to_string()),
            "{}",
            action
        );
        self.add(
            tx,
            AuditLogEntry::new(
            actor,
            target,
            action.to_owned(),
            details,
            ),
        )
        .await
    }

    async fn add(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: AuditLogEntry,
    ) -> Result<(), Error> {
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
            event.target_type as Option<TargetType>,
            event.actor_id,
            event.actor_type as ActorType,
            event.action,
            event.details,
            event.occurred_at,
        )
        .execute(&mut **tx)
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
            ORDER BY occurred_at DESC
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
        let mut tx = pool.begin().await.unwrap();

        repository
            .log(
                &mut tx,
                SYSTEM,
                (project_id, org_id),
                "test log",
                Some(serde_json::json!({"key": "value"})),
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let logs = repository.list(org_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].target_id, Some(*project_id));
        assert_eq!(logs[0].action, "test log");
        assert_eq!(
            logs[0].details.as_ref().unwrap().get("key").unwrap(),
            "value"
        );
    }
}
