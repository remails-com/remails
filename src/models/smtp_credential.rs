use crate::models::{Error, streams::StreamId};
use derive_more::{Deref, Display, From, FromStr};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, sqlx::Type, FromStr,
)]
#[sqlx(transparent)]
pub struct SmtpCredentialId(Uuid);

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredential {
    id: SmtpCredentialId,
    description: String,
    username: String,
    #[serde(skip)]
    password_hash: String,
    stream_id: StreamId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SmtpCredentialRequest {
    pub(crate) description: String,
    pub(crate) username: String,
    pub(crate) stream_id: StreamId,
}

#[derive(Serialize, derive_more::Debug)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredentialResponse {
    id: SmtpCredentialId,
    username: String,
    #[debug("****")]
    cleartext_password: String,
    stream_id: StreamId,
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
    ) -> Result<SmtpCredentialResponse, Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 20);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        let generated = sqlx::query_as!(
            SmtpCredential,
            r#"
            INSERT INTO smtp_credentials (id, description, username, password_hash, stream_id)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING *
            "#,
            new_credential.description,
            &new_credential.username,
            password_hash,
            *new_credential.stream_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SmtpCredentialResponse {
            id: generated.id,
            username: generated.username,
            cleartext_password: password,
            stream_id: generated.stream_id,
            created_at: generated.created_at,
            updated_at: generated.updated_at,
        })
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<SmtpCredential>, Error> {
        let credential = sqlx::query_as!(
            SmtpCredential,
            r#"
            SELECT * FROM smtp_credentials WHERE username = $1 LIMIT 1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(credential)
    }

    pub async fn list(&self) -> Result<Vec<SmtpCredential>, Error> {
        let credentials = sqlx::query_as!(
            SmtpCredential,
            r#"
            SELECT * FROM smtp_credentials ORDER BY created_at DESC
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

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "domains", "projects", "streams")
    ))]
    async fn smtp_credential_repository(pool: PgPool) {
        let credential_request = SmtpCredentialRequest {
            username: "test".to_string(),
            stream_id: "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap(),
            description: "Test SMTP credential description".to_string(),
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
