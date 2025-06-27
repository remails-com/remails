use crate::{
    models::{ApiUserId, Error},
    moneybird::MoneybirdContactId,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Organization {
    id: OrganizationId,
    pub name: String,
    remaining_message_quota: i64,
    quota_reset: DateTime<Utc>,
    moneybird_contact_id: Option<MoneybirdContactId>,
    remaining_rate_limit: i64,
    rate_limit_reset: DateTime<Utc>,
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
    name: String,
}

#[derive(Clone)]
pub struct OrganizationRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, Deserialize, Default)]
pub struct OrganizationFilter {
    pub(crate) orgs: Option<Vec<OrganizationId>>,
}

impl OrganizationFilter {
    fn org_uuids(&self) -> Option<Vec<Uuid>> {
        self.orgs
            .as_deref()
            .map(|o| o.iter().map(|o| o.as_uuid()).collect())
    }
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

    async fn reset_quota(&self, id: OrganizationId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE organizations
            SET quota_reset = quota_reset + '1 day', --TODO align the start/end date with the subscription period
                remaining_message_quota = 30 -- TODO make quota subscription dependent
            WHERE id = $1
            "#,
            *id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn reduce_quota(&self, id: OrganizationId) -> Result<QuotaStatus, Error> {
        struct Quota {
            remaining_message_quota: i64,
            quota_reset: DateTime<Utc>,
        }

        let quota = sqlx::query_as!(
            Quota,
            r#"
            UPDATE organizations
            SET remaining_message_quota =
                CASE WHEN remaining_message_quota - 1 > 0
                    THEN remaining_message_quota - 1
                    ELSE 0
                END
            WHERE id = $1
            RETURNING remaining_message_quota, quota_reset
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;

        if quota.quota_reset < Utc::now() {
            self.reset_quota(id).await?;
            // Redo the check with the refilled quota
            return Box::pin(self.reduce_quota(id)).await;
        }

        if quota.remaining_message_quota <= 0 {
            Ok(QuotaStatus::Exceeded)
        } else {
            Ok(QuotaStatus::Below(quota.remaining_message_quota as u64))
        }
    }

    pub async fn create(&self, organization: NewOrganization) -> Result<Organization, Error> {
        Ok(sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (id, name, remaining_message_quota, quota_reset, remaining_rate_limit, rate_limit_reset)
            VALUES (gen_random_uuid(), $1, 50, now() + '1 month', 0, now())
            RETURNING id,
                      name,
                      remaining_message_quota,
                      quota_reset,
                      created_at,
                      updated_at,
                      moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                      rate_limit_reset,
                      remaining_rate_limit
            "#,
            organization.name,
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list(&self, filter: &OrganizationFilter) -> Result<Vec<Organization>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            Organization,
            r#"
            SELECT id,
                   name,
                   remaining_message_quota,
                   quota_reset,
                   created_at,
                   updated_at,
                   moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                   rate_limit_reset,
                   remaining_rate_limit
            FROM organizations
            WHERE ($1::uuid[] IS NULL OR id = ANY($1))
            ORDER BY name
            "#,
            orgs.as_deref(),
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_by_id(
        &self,
        id: OrganizationId,
        filter: &OrganizationFilter,
    ) -> Result<Option<Organization>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            Organization,
            r#"
            SELECT id,
                   name,
                   remaining_message_quota,
                   quota_reset,
                   created_at,
                   updated_at,
                   moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId",
                   rate_limit_reset,
                   remaining_rate_limit
            FROM organizations
            WHERE id = $1
              AND ($2::uuid[] IS NULL OR id = ANY($2))
            "#,
            *id,
            orgs.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        id: OrganizationId,
        filter: &OrganizationFilter,
    ) -> Result<OrganizationId, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM organizations
            WHERE id = $1
              AND ($2::uuid[] IS NULL OR id = ANY($2))
            RETURNING id
            "#,
            *id,
            orgs.as_deref(),
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    pub async fn add_user(&self, org_id: OrganizationId, user_id: ApiUserId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id, role)
            VALUES ($1, $2, 'admin')
            "#,
            *org_id,
            *user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    impl Organization {
        pub fn message_quota(&self) -> i64 {
            self.remaining_message_quota
        }

        pub fn quota_reset(&self) -> DateTime<Utc> {
            self.quota_reset
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

        let orgs = repo.list(&Default::default()).await.unwrap();
        assert_eq!(orgs, vec![org1.clone(), org2.clone()]);

        repo.remove(org1.id, &Default::default()).await.unwrap();
        let orgs = repo.list(&Default::default()).await.unwrap();
        assert_eq!(orgs, vec![org2.clone()]);

        let not_found = repo.get_by_id(org1.id, &Default::default()).await.unwrap();
        assert_eq!(None, not_found);
    }
}
