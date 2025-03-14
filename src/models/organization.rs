use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Organization {
    pub id: Uuid,
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
    pub(crate) api_user_id: Option<Uuid>,
}

impl OrganizationRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, organization: NewOrganization) -> Result<Organization, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (id, name)
            VALUES (gen_random_uuid(), $1)
            RETURNING *
            "#,
            organization.name,
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list(
        &self,
        filter: &OrganizationFilter,
    ) -> Result<Vec<Organization>, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"
            SELECT DISTINCT o.* FROM organizations o
                LEFT JOIN api_users_organizations a ON o.id = a.organization_id
            WHERE ($1::uuid IS NULL OR a.api_user_id = $1)
            ORDER BY o.name
            "#,
            filter.api_user_id,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_by_id(
        &self,
        id: Uuid,
        filter: &OrganizationFilter,
    ) -> Result<Option<Organization>, sqlx::Error> {
        sqlx::query_as!(
            Organization,
            r#"
            SELECT DISTINCT o.* FROM organizations  o
                JOIN api_users_organizations a ON o.id = a.organization_id
            WHERE id = $1
              AND ($2::uuid IS NULL OR a.api_user_id = $2)
            "#,
            id,
            filter.api_user_id,
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn remove(&self, id: Uuid, filter: &OrganizationFilter) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM organizations
            USING organizations o
                LEFT JOIN api_users_organizations a ON o.id = a.organization_id
            WHERE o.id = organizations.id
              AND o.id = $1
              AND ($2::uuid IS NULL OR a.api_user_id = $2)
            "#,
            id,
            filter.api_user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_user(&self, org_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id)
            VALUES ($1, $2)
            "#,
            org_id,
            user_id
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
