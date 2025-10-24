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

#[derive(Serialize, derive_more::Debug)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredential {
    id: SmtpCredentialId,
    description: String,
    username: String,
    #[serde(skip)]
    #[debug("******")]
    password_hash: String,
    stream_id: StreamId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct SmtpCredentialRequest {
    pub(crate) description: String,
    pub(crate) username: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct SmtpCredentialUpdateRequest {
    pub(crate) description: String,
}

#[derive(Serialize, derive_more::Debug)]
#[cfg_attr(test, derive(Deserialize))]
pub struct SmtpCredentialResponse {
    id: SmtpCredentialId,
    description: String,
    username: String,
    #[debug("****")]
    cleartext_password: String,
    stream_id: StreamId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SmtpCredentialResponse {
    pub fn id(&self) -> SmtpCredentialId {
        self.id
    }

    pub fn username(&self) -> String {
        self.username.clone()
    }

    #[cfg(test)]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[cfg(test)]
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

impl SmtpCredential {
    pub fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub fn id(&self) -> SmtpCredentialId {
        self.id
    }

    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    #[cfg(test)]
    pub fn description(&self) -> &str {
        &self.description
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
            description: generated.description,
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

    pub async fn update(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        credential_id: SmtpCredentialId,
        update: &SmtpCredentialUpdateRequest,
    ) -> Result<SmtpCredential, Error> {
        Ok(sqlx::query_as!(
            SmtpCredential,
            r#"
            UPDATE smtp_credentials cred
            SET description = $1
            FROM streams s
                JOIN projects p ON s.project_id = p.id
            WHERE cred.id = $2
              AND cred.stream_id = s.id
              AND cred.stream_id = $3
              AND s.project_id = $4
              AND p.organization_id = $5
            RETURNING
                cred.id,
                cred.stream_id,
                cred.updated_at,
                cred.created_at,
                '' AS "password_hash!",
                cred.description,
                cred.username
            "#,
            update.description,
            *credential_id,
            *stream_id,
            *project_id,
            *org_id,
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        org_id: OrganizationId,
        project_id: ProjectId,
        stream_id: StreamId,
        credential_id: SmtpCredentialId,
    ) -> Result<SmtpCredentialId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM smtp_credentials c
                   USING streams s
                       JOIN projects p ON s.project_id = p.id
                   WHERE c.stream_id = s.id
                     AND p.organization_id = $1
                     AND p.id = $2
                     AND s.id = $3
                     AND c.id = $4
            RETURNING c.id
            "#,
            *org_id,
            *project_id,
            *stream_id,
            *credential_id,
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{models::MessageRepository, test::TestStreams};
    use sqlx::PgPool;

    impl SmtpCredentialResponse {
        pub fn cleartext_password(&self) -> String {
            self.cleartext_password.clone()
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "projects", "streams")))]
    async fn generate_happy_flow(pool: PgPool) {
        let credential_request = SmtpCredentialRequest {
            username: "test".to_string(),
            description: "Test SMTP credential description".to_string(),
        };
        let credential_repo = SmtpCredentialRepository::new(pool.clone());

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();

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

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn remove_happy_flow(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let rm_cred = credential_repo
            .remove(org_id, project_id, stream_id, credential_id)
            .await
            .unwrap();
        assert_eq!(credential_id, rm_cred);

        let not_found = credential_repo.find_by_username("marc").await.unwrap();
        assert!(not_found.is_none())
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn remove_org_does_not_match_proj(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (_org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let org_id = TestStreams::Org2Project1Stream1.org_id();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let not_found = credential_repo
            .remove(org_id, project_id, stream_id, credential_id)
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let still_there = credential_repo.find_by_username("marc").await.unwrap();
        assert!(still_there.is_some())
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn remove_proj_does_not_match_stream(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (org_id, _project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let project_id = TestStreams::Org2Project1Stream1.project_id();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let not_found = credential_repo
            .remove(org_id, project_id, stream_id, credential_id)
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let org_id = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap();

        let not_found = credential_repo
            .remove(org_id, project_id, stream_id, credential_id)
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let still_there = credential_repo.find_by_username("marc").await.unwrap();
        assert!(still_there.is_some())
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials", "messages")
    ))]
    async fn remove_with_depending_messages(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());
        let message_repo = MessageRepository::new(db.clone());

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let message_id = "e165562a-fb6d-423b-b318-fd26f4610634".parse().unwrap();

        let message = message_repo
            .find_by_id(org_id, project_id, stream_id, message_id)
            .await
            .unwrap();

        // Making sure we are actually deleting a credential that has a message associated
        assert_eq!(message.smtp_credential_id(), Some(credential_id));

        // Deleting the credential
        let rm_cred = credential_repo
            .remove(org_id, project_id, stream_id, credential_id)
            .await
            .unwrap();
        assert_eq!(credential_id, rm_cred);

        // Making sure the credential is actually gone
        let not_found = credential_repo.find_by_username("marc").await.unwrap();
        assert!(not_found.is_none());

        // Making sure the message is still there
        let message = message_repo
            .find_by_id(org_id, project_id, stream_id, message_id)
            .await
            .unwrap();

        // And has no credential associated anymore
        assert_eq!(message.smtp_credential_id(), None);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn update_happy_flow(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let update = credential_repo
            .update(
                org_id,
                project_id,
                stream_id,
                credential_id,
                &SmtpCredentialUpdateRequest {
                    description: "Updated description".to_string(),
                },
            )
            .await
            .unwrap();
        assert_eq!(credential_id, update.id);
        assert_eq!("Updated description", update.description);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn update_org_does_not_match_proj(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (_org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let org_id = TestStreams::Org2Project1Stream1.org_id();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let not_found = credential_repo
            .update(
                org_id,
                project_id,
                stream_id,
                credential_id,
                &SmtpCredentialUpdateRequest {
                    description: "Should not work".to_string(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "streams", "smtp_credentials")
    ))]
    async fn update_proj_does_not_match_stream(db: PgPool) {
        let credential_repo = SmtpCredentialRepository::new(db.clone());

        let (org_id, _project_id, stream_id) = TestStreams::Org1Project1Stream1.get_ids();
        let project_id = TestStreams::Org2Project1Stream1.project_id();
        let credential_id = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf".parse().unwrap();

        let not_found = credential_repo
            .update(
                org_id,
                project_id,
                stream_id,
                credential_id,
                &SmtpCredentialUpdateRequest {
                    description: "Should not work".to_string(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));
    }
}
