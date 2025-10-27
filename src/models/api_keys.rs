use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::{Error, OrganizationId, Password, Role};

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, ToSchema,
)]
pub struct ApiKeyId(Uuid);

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize, Debug))]
pub struct ApiKey {
    id: ApiKeyId,
    description: String,
    #[serde(skip)]
    password_hash: String,
    organization_id: OrganizationId,
    role: Role,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ApiKey {
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    pub fn role(&self) -> &Role {
        &self.role
    }

    pub fn verify_password(&self, password: &Password) -> bool {
        password.verify_password(&self.password_hash).is_ok()
    }
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize, Debug))]
pub struct CreatedApiKeyWithPassword {
    id: ApiKeyId,
    description: String,
    password: String,
    organization_id: OrganizationId,
    role: Role,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CreatedApiKeyWithPassword {
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    #[cfg(test)]
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    #[cfg(test)]
    pub fn role(&self) -> &Role {
        &self.role
    }

    #[cfg(test)]
    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct ApiKeyRequest {
    pub description: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        ApiKeyRepository { pool }
    }

    pub async fn create(
        &self,
        org_id: OrganizationId,
        key: &ApiKeyRequest,
    ) -> Result<CreatedApiKeyWithPassword, Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        if key.role.is_at_least(Role::Admin) {
            return Err(Error::BadRequest(format!(
                "Can't create API key with {} role",
                key.role
            )));
        }

        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            INSERT INTO api_keys (id, description, password_hash, organization_id, role)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING id, description, password_hash, organization_id, role as "role: Role", created_at, updated_at
            "#,
            key.description,
            password_hash,
            *org_id,
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

    pub async fn get(&self, key_id: ApiKeyId) -> Result<ApiKey, Error> {
        Ok(sqlx::query_as!(
            ApiKey,
            r#"
            SELECT id, description, password_hash, organization_id, role as "role: Role", created_at, updated_at
            FROM api_keys
            WHERE id = $1
            "#,
            *key_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list(&self, org_id: OrganizationId) -> Result<Vec<ApiKey>, Error> {
        Ok(sqlx::query_as!(
            ApiKey,
            r#"
            SELECT id, description, password_hash, organization_id, role as "role: Role", created_at, updated_at
            FROM api_keys
            WHERE organization_id = $1
            "#,
            *org_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn update(
        &self,
        org_id: OrganizationId,
        key_id: ApiKeyId,
        changes: &ApiKeyRequest,
    ) -> Result<ApiKey, Error> {
        if changes.role.is_at_least(Role::Admin) {
            return Err(Error::BadRequest(format!(
                "Can't update API key to {} role",
                changes.role
            )));
        }

        Ok(sqlx::query_as!(
            ApiKey,
            r#"
            UPDATE api_keys
            SET description = $1, role = $2
            WHERE organization_id = $3 AND id = $4
            RETURNING id, description, password_hash, organization_id, role as "role: Role", created_at, updated_at
            "#,
            changes.description,
            changes.role as Role,
            *org_id,
            *key_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        org_id: OrganizationId,
        key_id: ApiKeyId,
    ) -> Result<ApiKeyId, Error> {
        let id = sqlx::query_scalar!(
            r#"
            DELETE FROM api_keys
            WHERE organization_id = $1 AND id = $2
            RETURNING id
            "#,
            *org_id,
            *key_id,
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
        let repo = ApiKeyRepository::new(db);
        let org_id: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1

        // create API key
        let new = ApiKeyRequest {
            description: "MyKey".to_string(),
            role: Role::Maintainer,
        };
        let api_key = repo.create(org_id, &new).await.unwrap();
        assert_eq!(api_key.description, new.description);
        assert_eq!(api_key.organization_id, org_id);
        assert_eq!(api_key.role, new.role);

        // list API keys
        let api_keys = repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id, api_key.id);
        assert_eq!(api_keys[0].organization_id, org_id);
        assert_eq!(api_keys[0].description, api_key.description);
        assert_eq!(api_keys[0].role, api_key.role);

        // update API key
        let update = ApiKeyRequest {
            description: "UpdatedKey".to_string(),
            role: Role::ReadOnly,
        };
        let id = *api_key.id();
        let api_key = repo.update(org_id, id, &update).await.unwrap();
        assert_eq!(api_key.description, update.description);
        assert_eq!(api_key.id, id);
        assert_eq!(api_key.organization_id, org_id);
        assert_eq!(api_key.role, update.role);

        // list API keys
        let api_keys = repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id, api_key.id);
        assert_eq!(api_keys[0].organization_id, org_id);
        assert_eq!(api_keys[0].description, update.description);
        assert_eq!(api_keys[0].role, update.role);

        // remove API key
        let removed_id = repo.remove(org_id, api_key.id).await.unwrap();
        assert_eq!(removed_id, api_key.id);

        // verify that key was removed
        let api_keys = repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 0);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "api_keys")
    ))]
    async fn no_admin_api_keys(db: PgPool) {
        let repo = ApiKeyRepository::new(db);
        let org_id: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1

        // we should not allow creating admin-level API keys
        let err = repo
            .create(
                org_id,
                &ApiKeyRequest {
                    description: "Admin?".to_string(),
                    role: Role::Admin,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));

        // we should not allow updating to admin-level API keys
        let key_id = "951ec618-bcc9-4224-9cf1-ed41a84f41d8".parse().unwrap(); // maintainer API key in org 1
        let err = repo
            .update(
                org_id,
                key_id,
                &ApiKeyRequest {
                    description: "Admin?".to_string(),
                    role: Role::Admin,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));

        // list API keys
        let api_keys = repo.list(org_id).await.unwrap();
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id, key_id);
        assert_eq!(api_keys[0].organization_id, org_id);
        assert_eq!(api_keys[0].description, "Test API key unknown password");
        assert_eq!(api_keys[0].role, Role::Maintainer);
    }
}
