use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SmtpCredential {
    id: Uuid,
    username: String,
    password_hash: String,
    domain_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SmtpCredential {
    pub fn new(username: String, password: String, domain_id: Uuid) -> Self {
        let password_hash = password_auth::generate_hash(password.as_bytes());

        Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            domain_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub fn id(&self) -> Uuid {
        self.id
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

    pub async fn create(
        &self,
        new_credential: &SmtpCredential,
    ) -> Result<SmtpCredential, sqlx::Error> {
        let credential: SmtpCredential = sqlx::query_as!(
            SmtpCredential,
            r#"
            INSERT INTO smtp_credential (id, username, password_hash, domain_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
            new_credential.id,
            new_credential.username,
            new_credential.password_hash,
            new_credential.domain_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(credential)
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
        let repository = SmtpCredentialRepository::new(pool);

        let credential = SmtpCredential::new(
            "test".into(),
            "password".into(),
            "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        );
        assert!(credential.verify_password("password"));

        repository.create(&credential).await.unwrap();

        let fetched_user = repository.find_by_username("test").await.unwrap().unwrap();

        assert_eq!(fetched_user.username, "test");
        assert!(fetched_user.verify_password("password"));
    }
}
