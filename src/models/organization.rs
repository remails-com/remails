use crate::models::{ApiUserId, Error};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct OrganizationId(Uuid);

impl OrganizationId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewOrganization {
    name: String,
}

#[derive(Debug, Clone)]
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

impl OrganizationRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, organization: NewOrganization) -> Result<Organization, Error> {
        Ok(sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (id, name)
            VALUES (gen_random_uuid(), $1)
            RETURNING *
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
            SELECT * FROM organizations
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
            SELECT * FROM organizations
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
    ) -> Result<(), Error> {
        let orgs = filter.org_uuids();
        sqlx::query!(
            r#"
            DELETE FROM organizations
            WHERE id = $1
              AND ($2::uuid[] IS NULL OR id = ANY($2))
            "#,
            *id,
            orgs.as_deref(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_user(&self, org_id: OrganizationId, user_id: ApiUserId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id)
            VALUES ($1, $2)
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
