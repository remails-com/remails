use crate::{
    models::{ApiUserId, Error, Role},
    moneybird::{MoneybirdContactId, SubscriptionStatus},
};
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
    FromStr,
    sqlx::Type,
    PartialOrd,
    Ord,
    Eq,
)]
#[sqlx(transparent)]
pub struct OrganizationId(Uuid);

impl OrganizationId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[cfg_attr(test, derive(Clone, Deserialize))]
pub struct Organization {
    id: OrganizationId,
    pub name: String,
    total_message_quota: i64,
    used_message_quota: i64,
    quota_reset: Option<DateTime<Utc>>,
    moneybird_contact_id: Option<MoneybirdContactId>,
    remaining_rate_limit: i64,
    rate_limit_reset: DateTime<Utc>,
    current_subscription: SubscriptionStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Organization {
    pub fn id(&self) -> OrganizationId {
        self.id
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewOrganization {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct OrganizationMember {
    user_id: ApiUserId,
    email: String,
    name: String,
    role: Role,
    added_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OrganizationMember {
    pub fn user_id(&self) -> &ApiUserId {
        &self.user_id
    }

    pub fn role(&self) -> &Role {
        &self.role
    }
}

struct PgOrganization {
    id: OrganizationId,
    pub name: String,
    total_message_quota: i64,
    used_message_quota: i64,
    quota_reset: Option<DateTime<Utc>>,
    moneybird_contact_id: Option<MoneybirdContactId>,
    remaining_rate_limit: i64,
    rate_limit_reset: DateTime<Utc>,
    current_subscription: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgOrganization> for Organization {
    type Error = serde_json::Error;

    fn try_from(pg: PgOrganization) -> Result<Self, Self::Error> {
        Ok(Self {
            id: pg.id,
            name: pg.name,
            total_message_quota: pg.total_message_quota,
            used_message_quota: pg.used_message_quota,
            quota_reset: pg.quota_reset,
            moneybird_contact_id: pg.moneybird_contact_id,
            remaining_rate_limit: pg.remaining_rate_limit,
            rate_limit_reset: pg.rate_limit_reset,
            current_subscription: serde_json::from_value(pg.current_subscription)?,
            created_at: pg.created_at,
            updated_at: pg.updated_at,
        })
    }
}

#[derive(Clone)]
pub struct OrganizationRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QuotaStatus {
    Exceeded,
    Below(u64),
}

impl OrganizationRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn reduce_quota(&self, id: OrganizationId) -> Result<QuotaStatus, Error> {
        let quota = sqlx::query_scalar!(
            r#"
            UPDATE organizations
            SET used_message_quota = LEAST(used_message_quota + 1, total_message_quota)
            WHERE id = $1
            RETURNING (total_message_quota - used_message_quota) as "remaining!"
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;

        if quota <= 0 {
            Ok(QuotaStatus::Exceeded)
        } else {
            Ok(QuotaStatus::Below(quota as u64))
        }
    }

    pub async fn create(&self, organization: NewOrganization) -> Result<Organization, Error> {
        Ok(sqlx::query_as!(
            PgOrganization,
            r#"
            INSERT INTO organizations (id, name, total_message_quota, used_message_quota, quota_reset, remaining_rate_limit, rate_limit_reset)
            VALUES (gen_random_uuid(), $1, 50, 0, now(), 0, now())
            RETURNING id,
                      name,
                      total_message_quota,
                      used_message_quota,
                      quota_reset,
                      created_at,
                      updated_at,
                      moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                      rate_limit_reset,
                      remaining_rate_limit,
                      current_subscription
            "#,
            organization.name.trim(),
        )
            .fetch_one(&self.pool)
            .await?
            .try_into()?)
    }

    pub async fn update(
        &self,
        id: OrganizationId,
        organization: NewOrganization,
    ) -> Result<Organization, Error> {
        Ok(sqlx::query_as!(
            PgOrganization,
            r#"
            UPDATE organizations
            SET name = $2
            WHERE id = $1
            RETURNING
                id,
                name,
                total_message_quota,
                used_message_quota,
                quota_reset,
                created_at,
                updated_at,
                moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                rate_limit_reset,
                remaining_rate_limit,
                current_subscription
            "#,
            *id,
            organization.name.trim(),
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()?)
    }

    pub async fn list(&self, filter: Option<Vec<Uuid>>) -> Result<Vec<Organization>, Error> {
        Ok(sqlx::query_as!(
            PgOrganization,
            r#"
            SELECT id,
                   name,
                   total_message_quota,
                   used_message_quota,
                   quota_reset,
                   created_at,
                   updated_at,
                   moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                   rate_limit_reset,
                   remaining_rate_limit,
                   current_subscription
            FROM organizations
            WHERE ($1::uuid[] IS NULL OR id = ANY($1))
            ORDER BY name
            "#,
            filter.as_deref(),
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::<Organization>::try_into)
        .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn get_by_id(&self, id: OrganizationId) -> Result<Option<Organization>, Error> {
        Ok(sqlx::query_as!(
            PgOrganization,
            r#"
            SELECT id,
                   name,
                   total_message_quota,
                   used_message_quota,
                   quota_reset,
                   created_at,
                   updated_at,
                   moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                   rate_limit_reset,
                   remaining_rate_limit,
                   current_subscription
            FROM organizations
            WHERE id = $1
            "#,
            *id,
        )
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::<Organization>::try_into)
        .transpose()?)
    }

    pub async fn remove(&self, id: OrganizationId) -> Result<OrganizationId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM organizations
            WHERE id = $1
            RETURNING id
            "#,
            *id,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    pub async fn add_user(
        &self,
        org_id: OrganizationId,
        user_id: ApiUserId,
        role: Role,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id, role)
            VALUES ($1, $2, $3)
            "#,
            *org_id,
            *user_id,
            role as Role
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_members(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<OrganizationMember>, Error> {
        Ok(sqlx::query_as!(
            OrganizationMember,
            r#"
            SELECT o.api_user_id as "user_id", u.email, u.name, o.role as "role: Role", o.created_at as "added_at", o.updated_at
            FROM api_users_organizations o
            JOIN api_users u ON o.api_user_id = u.id
            WHERE o.organization_id = $1
            "#,
            *org_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn remove_member(
        &self,
        org_id: OrganizationId,
        user_id: ApiUserId,
    ) -> Result<ApiUserId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM api_users_organizations
            WHERE organization_id = $1 AND api_user_id = $2
            RETURNING api_user_id
            "#,
            *org_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    impl Organization {
        pub fn remaining_message_quota(&self) -> i64 {
            self.total_message_quota - self.used_message_quota
        }

        pub fn quota_reset(&self) -> Option<DateTime<Utc>> {
            self.quota_reset
        }

        pub fn total_message_quota(&self) -> i64 {
            self.total_message_quota
        }

        pub fn current_subscription(&self) -> &SubscriptionStatus {
            &self.current_subscription
        }
    }

    #[sqlx::test]
    async fn organization_lifecycle(db: PgPool) {
        let repo = OrganizationRepository::new(db);

        let org1 = repo
            .create(NewOrganization {
                name: "TestOrg1".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(org1.name, "TestOrg1");
        let org2 = repo
            .create(NewOrganization {
                name: "TestOrg2".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(org2.name, "TestOrg2");

        let orgs = repo.list(None).await.unwrap();
        assert_eq!(orgs, vec![org1.clone(), org2.clone()]);

        repo.remove(org1.id).await.unwrap();
        let orgs = repo.list(None).await.unwrap();
        assert_eq!(orgs, vec![org2.clone()]);

        let not_found = repo.get_by_id(org1.id).await.unwrap();
        assert_eq!(None, not_found);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn organization_remove_member(db: PgPool) {
        let org_2 = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap();
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2

        let repo = OrganizationRepository::new(db);

        // org 2 contains two members: user_1 and user_2
        let members = repo.list_members(org_2).await.unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|m| m.user_id == user_1));
        assert!(members.iter().any(|m| m.user_id == user_2));

        // remove user_1 from org 2
        let removed_user = repo.remove_member(org_2, user_1).await.unwrap();
        assert_eq!(removed_user, user_1);

        // now org 2 should only contain 1 member: user_2
        let members = repo.list_members(org_2).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, user_2);
    }
}
