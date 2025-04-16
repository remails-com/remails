use crate::models::{Error, OrganizationId, ProjectId, streams::StreamId};
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
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        new_credential: &SmtpCredentialRequest,
    ) -> Result<SmtpCredentialResponse, Error> {
        sqlx::query_scalar!(
            r#"
            SELECT s.id FROM streams s 
                JOIN projects p ON s.project_id = p.id
                JOIN organizations o ON p.organization_id = o.id
            WHERE o.id = $1 
              AND p.id = $2 
              AND s.id = $3
            "#,
            *org_id,
            *project_id,
            *stream_id,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::BadRequest(
            "The stream does not exist or it does not match the provided organization or project"
                .to_string(),
        ))?;

        // Prepend the requested username with the beginning of the organization UUID
        // to ensure global uniqueness
        let username = format!("{}-{}", &org_id.to_string()[0..8], new_credential.username);

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
            username,
            password_hash,
            *stream_id
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
            SELECT * FROM smtp_credentials WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(credential)
    }

    pub async fn list(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
    ) -> Result<Vec<SmtpCredential>, Error> {
        let credentials = sqlx::query_as!(
            SmtpCredential,
            r#"
            SELECT c.* FROM smtp_credentials c
                JOIN streams s ON c.stream_id = s.id
                JOIN projects p ON s.project_id = p.id
            WHERE p.organization_id = $1 
              AND p.id = $2 
              AND s.id = $3
            ORDER BY c.created_at DESC
            "#,
            *org_id,
            *project_id,
            *stream_id,
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

    impl SmtpCredentialResponse {
        pub fn id(&self) -> SmtpCredentialId {
            self.id
        }

        pub fn cleartext_password(&self) -> String {
            self.cleartext_password.clone()
        }

        pub fn username(&self) -> String {
            self.username.clone()
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    async fn smtp_credential_repository(pool: PgPool) {
        let credential_request = SmtpCredentialRequest {
            username: "test".to_string(),
            description: "Test SMTP credential description".to_string(),
        };
        let credential_repo = SmtpCredentialRepository::new(pool.clone());

        let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

        let credential = credential_repo
            .generate(org_id, project_id, stream_id, &credential_request)
            .await
            .unwrap();

        assert_ne!(credential_request.username, credential.username);
        assert!(
            credential
                .username
                .ends_with(credential_request.username.as_str())
        );

        let get_credential = credential_repo
            .find_by_username(&credential.username)
            .await
            .unwrap()
            .unwrap();

        assert!(get_credential.verify_password(credential.cleartext_password.as_str()));
    }
}
