use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{ApiUserId, Error, OrganizationId};

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, sqlx::Type, FromStr,
)]
#[sqlx(transparent)]
pub struct InviteId(Uuid);

#[derive(Serialize)]
pub struct CreatedInvite {
    id: InviteId,
    password_hash: String,
    organization_id: OrganizationId,
    created_by: ApiUserId,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CreatedInviteWithPassword {
    id: InviteId,
    password: String,
    organization_id: OrganizationId,
    created_by: ApiUserId,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ApiInvite {
    id: InviteId,
    organization_id: OrganizationId,
    organization_name: String,
    #[serde(skip)]
    password_hash: String,
    created_by: ApiUserId,
    created_by_name: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl ApiInvite {
    pub fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}

#[derive(Debug, Clone)]
pub struct InviteRepository {
    pool: sqlx::PgPool,
}

impl InviteRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        org_id: OrganizationId,
        created_by: ApiUserId,
        expires: DateTime<Utc>,
    ) -> Result<CreatedInviteWithPassword, Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        let invite = sqlx::query_as!(
            CreatedInvite,
            r#"
            INSERT INTO organization_invites (id, password_hash, organization_id, created_by, expires_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING *
            "#,
            password_hash,
            *org_id,
            *created_by,
            expires
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(CreatedInviteWithPassword {
            id: invite.id,
            password,
            organization_id: invite.organization_id,
            created_by: invite.created_by,
            created_at: invite.created_at,
            expires_at: invite.expires_at,
        })
    }

    pub async fn get_by_org(&self, org_id: OrganizationId) -> Result<Vec<ApiInvite>, Error> {
        Ok(sqlx::query_as!(
            ApiInvite,
            r#"
            SELECT i.id, i.organization_id, o.name AS organization_name,
                i.password_hash,
                i.created_by, a.name AS created_by_name, 
                i.created_at, i.expires_at
            FROM organization_invites i
            JOIN organizations o ON o.id = i.organization_id
            JOIN api_users a ON a.id = i.created_by
            WHERE i.organization_id = $1
            "#,
            *org_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_by_id(
        &self,
        invite_id: InviteId,
        org_id: OrganizationId,
    ) -> Result<ApiInvite, Error> {
        Ok(sqlx::query_as!(
            ApiInvite,
            r#"
            SELECT i.id, i.organization_id, o.name AS organization_name, 
                i.password_hash,
                i.created_by, a.name AS created_by_name, 
                i.created_at, i.expires_at
            FROM organization_invites i
            JOIN organizations o ON o.id = i.organization_id
            JOIN api_users a ON a.id = i.created_by
            WHERE i.id = $1 AND i.organization_id = $2
            "#,
            *invite_id,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn remove_by_id(
        &self,
        invite_id: InviteId,
        org_id: OrganizationId,
    ) -> Result<InviteId, Error> {
        let id = sqlx::query_scalar!(
            r#"
            DELETE FROM organization_invites
            WHERE id = $1 AND organization_id = $2
            RETURNING id
            "#,
            *invite_id,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(InviteId(id))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn invite_lifecycle(db: PgPool) {
        let invite_repo = InviteRepository::new(db);
        let org_id: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1
        let created_by: ApiUserId = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // test user 1

        // create invite
        let invite = invite_repo
            .create(org_id, created_by, Utc::now() + chrono::Duration::days(3))
            .await
            .unwrap();

        let invites = invite_repo.get_by_org(org_id).await.unwrap();
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].id, invite.id);

        // invite retrieval
        let retrieved_invite = invite_repo.get_by_id(invite.id, org_id).await.unwrap();
        assert_eq!(retrieved_invite.id, invite.id);

        // wrong organization retrieval
        let org_id2: OrganizationId = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap();
        assert!(matches!(
            invite_repo.get_by_id(invite.id, org_id2).await,
            Err(Error::NotFound(_))
        ));

        // remove invite
        assert!(matches!(
            invite_repo.remove_by_id(invite.id, org_id2).await,
            Err(Error::NotFound(_))
        ));
        assert_eq!(
            invite_repo.remove_by_id(invite.id, org_id).await.unwrap(),
            invite.id,
        );

        let invites = invite_repo.get_by_org(org_id).await.unwrap();
        assert_eq!(invites.len(), 0);
    }
}
