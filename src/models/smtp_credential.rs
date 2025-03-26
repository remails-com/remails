use derive_more::{Deref, Display, From};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref)]
pub struct SmtpCredentialId(Uuid);

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredential {
    id: SmtpCredentialId,
    username: String,
    #[serde(skip)]
    password_hash: String,
    domain_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SmtpCredentialRequest {
    pub(crate) username: String,
    pub(crate) domain_id: Uuid,
}

#[derive(Serialize, derive_more::Debug)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredentialResponse {
    id: SmtpCredentialId,
    username: String,
    #[debug("****")]
    cleartext_password: String,
    domain_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SmtpCredential {
    pub fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub fn id(&self) -> SmtpCredentialId {
        self.id
    }
}

#[cfg(test)]
impl SmtpCredentialResponse {
    pub fn id(&self) -> SmtpCredentialId {
        self.id
    }

    pub fn cleartext_password(self) -> String {
        self.cleartext_password
    }
}

#[derive(Debug, Clone)]
pub struct SmtpCredentialRepository {
    pool: sqlx::PgPool,
}

impl SmtpCredentialRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn generate(
        &self,
        new_credential: &SmtpCredentialRequest,
    ) -> Result<SmtpCredentialResponse, sqlx::Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 20);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        let generated = sqlx::query_as!(
            SmtpCredential,
            r#"
            INSERT INTO smtp_credential (id, username, password_hash, domain_id)
            VALUES (gen_random_uuid(), $1, $2, $3)
            RETURNING *
            "#,
            &new_credential.username,
            password_hash,
            new_credential.domain_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SmtpCredentialResponse {
            id: generated.id,
            username: generated.username,
            cleartext_password: password,
            domain_id: generated.domain_id,
            created_at: generated.created_at,
            updated_at: generated.updated_at,
        })
    }

    pub async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<SmtpCredential>, sqlx::Error> {
        let credential = sqlx::query_as!(
            SmtpCredential,
            r#"
            SELECT * FROM smtp_credential WHERE username = $1 LIMIT 1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(credential)
    }

    pub async fn list(&self) -> Result<Vec<SmtpCredential>, sqlx::Error> {
        let credentials = sqlx::query_as!(
            SmtpCredential,
            r#"
            SELECT * FROM smtp_credential ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(credentials)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "domains")))]
    async fn smtp_credential_repository(pool: PgPool) {
        let credential_request = SmtpCredentialRequest {
            username: "test".to_string(),
            domain_id: "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        };
        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo.generate(&credential_request).await.unwrap();

        let get_credential = credential_repo
            .find_by_username("test")
            .await
            .unwrap()
            .unwrap();

        assert!(get_credential.verify_password(credential.cleartext_password.as_str()));
    }
}
