use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Error, OrganizationId, Role};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct ApiKeyId(Uuid);

#[derive(Serialize)]
pub struct ApiKey {
    id: ApiKeyId,
    description: String,
    organization_id: OrganizationId,
    role: Role,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CreatedApiKeyWithPassword {
    id: ApiKeyId,
    description: String,
    password: String,
    organization_id: OrganizationId,
    role: Role,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub struct NewApiKey {
    description: String,
    organization_id: OrganizationId,
    role: Role,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        ApiKeyRepository { pool }
    }

    pub async fn create(&self, key: &NewApiKey) -> Result<CreatedApiKeyWithPassword, Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            INSERT INTO api_keys (id, description, password_hash, organization_id, role)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING id, description, organization_id, role as "role: Role", created_at, updated_at
            "#,
            key.description,
            password_hash,
            *key.organization_id,
            key.role as Role
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(CreatedApiKeyWithPassword {
            id: api_key.id,
            description: api_key.description,
            password,
            organization_id: api_key.organization_id,
            role: api_key.role,
            created_at: api_key.created_at,
            updated_at: api_key.updated_at,
        })
    }

    pub async fn list(&self, org_id: OrganizationId) -> Result<Vec<ApiKey>, Error> {
        Ok(sqlx::query_as!(
            ApiKey,
            r#"
            SELECT id, description, organization_id, role as "role: Role", created_at, updated_at
            FROM api_keys
            WHERE organization_id = $1
            "#,
            *org_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        key_id: ApiKeyId,
        org_id: OrganizationId,
    ) -> Result<ApiKeyId, Error> {
        let id = sqlx::query_scalar!(
            r#"
            DELETE FROM api_keys
            WHERE id = $1 AND organization_id = $2
            RETURNING id
            "#,
            *key_id,
            *org_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ApiKeyId(id))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn api_key_lifecycle(db: PgPool) {
        let api_key_repo = ApiKeyRepository::new(db);
        let org_id: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1

        // create API key
        let new = NewApiKey {
            description: "MyKey".to_string(),
            organization_id: org_id,
            role: Role::Maintainer,
        };
        let api_key = api_key_repo.create(&new).await.unwrap();
        assert_eq!(api_key.description, new.description);
        assert_eq!(api_key.organization_id, new.organization_id);
        assert_eq!(api_key.role, new.role);

        // list API keys
        let api_keys = api_key_repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id, api_key.id);

        // remove API key
        let removed_id = api_key_repo.remove(api_key.id, org_id).await.unwrap();
        assert_eq!(removed_id, api_key.id);

        // verify that key was removed
        let api_keys = api_key_repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 0);
    }
}
