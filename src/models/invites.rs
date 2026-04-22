use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::Serialize;
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use crate::models::{Actor, ApiUserId, AuditLogRepository, Error, OrganizationId, Password, Role};

id!(
    #[derive(IntoParams)]
    #[into_params(names("invite_id"))]
    InviteId
);

/// Newly created invitation
///
/// Contains the password in plain text, which is only available at creation time
#[derive(Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct CreatedInviteWithPassword {
    id: InviteId,
    password: String,
    organization_id: OrganizationId,
    role: Role,
    created_by: ApiUserId,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl CreatedInviteWithPassword {
    #[cfg(test)]
    pub fn password(&self) -> &String {
        &self.password
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn id(&self) -> &InviteId {
        &self.id
    }

    #[cfg(test)]
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    #[cfg(test)]
    pub fn created_by(&self) -> &ApiUserId {
        &self.created_by
    }
}

#[derive(Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct ApiInvite {
    id: InviteId,
    organization_id: OrganizationId,
    organization_name: String,
    role: Role,
    #[serde(skip)]
    password_hash: String,
    created_by: ApiUserId,
    created_by_name: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl ApiInvite {
    pub fn verify_password(&self, password: &Password) -> bool {
        password.verify_password(&self.password_hash).is_ok()
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    pub fn role(&self) -> Role {
        self.role
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn id(&self) -> &InviteId {
        &self.id
    }

    #[cfg(test)]
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    #[cfg(test)]
    pub fn created_by(&self) -> &ApiUserId {
        &self.created_by
    }
}

#[derive(Debug, Clone)]
pub struct InviteRepository {
    pool: sqlx::PgPool,
    audit_log: AuditLogRepository,
}

impl InviteRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            audit_log: AuditLogRepository::new(pool.clone()),
            pool,
        }
    }

    pub async fn create(
        &self,
        org_id: OrganizationId,
        role: Role,
        created_by: ApiUserId,
        expires: DateTime<Utc>,
        actor: impl Into<Actor>,
    ) -> Result<CreatedInviteWithPassword, Error> {
        let password = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let password_hash = password_auth::generate_hash(password.as_bytes());

        let mut tx = self.pool.begin().await?;

        let invite = sqlx::query!(
            r#"
            INSERT INTO organization_invites (id, password_hash, organization_id, role, created_by, expires_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)
            RETURNING id, organization_id, role as "role: Role", created_by, created_at, expires_at
            "#,
            password_hash,
            *org_id,
            role as Role,
            *created_by,
            expires
        )
        .fetch_one(&mut *tx)
        .await?;

        self.audit_log
            .log(
                &mut tx,
                actor,
                (InviteId(invite.id), org_id),
                "Created invite link",
                Some(json!(role)),
            )
            .await?;

        tx.commit().await?;

        Ok(CreatedInviteWithPassword {
            id: invite.id.into(),
            password,
            organization_id: invite.organization_id.into(),
            role: invite.role,
            created_by: invite.created_by.into(),
            created_at: invite.created_at,
            expires_at: invite.expires_at,
        })
    }

    pub async fn get_by_org(&self, org_id: OrganizationId) -> Result<Vec<ApiInvite>, Error> {
        Ok(sqlx::query_as!(
            ApiInvite,
            r#"
            SELECT i.id, i.organization_id, o.name AS organization_name,
                i.role as "role: Role", i.password_hash,
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
                i.role as "role: Role", i.password_hash,
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
        actor: impl Into<Actor>,
    ) -> Result<InviteId, Error> {
        let mut tx = self.pool.begin().await?;
        let id: InviteId = sqlx::query_scalar!(
            r#"
            DELETE FROM organization_invites
            WHERE id = $1 AND organization_id = $2
            RETURNING id
            "#,
            *invite_id,
            *org_id
        )
        .fetch_one(&mut *tx)
        .await?
        .into();

        self.audit_log
            .log(&mut tx, actor, (id, org_id), "Deleted invite link", None)
            .await?;

        tx.commit().await?;

        Ok(id)
    }

    pub async fn accept(
        &self,
        invite_id: InviteId,
        org_id: OrganizationId,
        user_id: ApiUserId,
        role: Role,
        actor: impl Into<Actor>,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id, role)
            VALUES ($1, $2, $3)
            "#,
            *org_id,
            *user_id,
            role as Role
        )
        .execute(&mut *tx)
        .await?;

        let _: InviteId = sqlx::query_scalar!(
            r#"
            DELETE FROM organization_invites
            WHERE id = $1 AND organization_id = $2
            RETURNING id
            "#,
            *invite_id,
            *org_id
        )
        .fetch_one(&mut *tx)
        .await?
        .into();

        self.audit_log
            .log(
                &mut tx,
                actor,
                (invite_id, org_id),
                "Accepted invite link",
                Some(json!(role)),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn remove_expired_before(&self, before: DateTime<Utc>) -> Result<(), Error> {
        tracing::trace!("Removing expired invites before {before}");
        let rows = sqlx::query!(
            r#"
            DELETE FROM organization_invites
            WHERE expires_at < $1
            "#,
            before
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows > 0 {
            tracing::debug!("Removed {rows} expired invites");
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::{AuditLogRepository, SYSTEM};
    use sqlx::PgPool;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn invite_lifecycle(db: PgPool) {
        let invite_repo = InviteRepository::new(db.clone());
        let audit_log = AuditLogRepository::new(db);
        let org_id: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1
        let created_by: ApiUserId = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // test user 1

        // create invite
        let invite = invite_repo
            .create(
                org_id,
                Role::Admin,
                created_by,
                Utc::now() + chrono::Duration::days(3),
                SYSTEM,
            )
            .await
            .unwrap();

        let invites = invite_repo.get_by_org(org_id).await.unwrap();
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].id, invite.id);
        let audit_entries = audit_log.list(org_id).await.unwrap();
        assert_eq!(audit_entries.len(), 1);
        assert_eq!(audit_entries[0].target_id, Some(**invite.id()));
        assert_eq!(audit_entries[0].action, "Created invite link");

        // add expired invite
        invite_repo
            .create(org_id, Role::Admin, created_by, Utc::now(), SYSTEM)
            .await
            .unwrap();
        assert_eq!(invite_repo.get_by_org(org_id).await.unwrap().len(), 2);

        // remove expired invite
        invite_repo.remove_expired_before(Utc::now()).await.unwrap();
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
            invite_repo.remove_by_id(invite.id, org_id2, SYSTEM).await,
            Err(Error::NotFound(_))
        ));
        assert_eq!(
            invite_repo
                .remove_by_id(invite.id, org_id, SYSTEM)
                .await
                .unwrap(),
            invite.id,
        );
        let audit_entries = audit_log.list(org_id).await.unwrap();
        assert_eq!(audit_entries.len(), 3);
        assert_eq!(audit_entries[0].target_id, Some(**invite.id()));
        assert_eq!(audit_entries[0].action, "Deleted invite link");

        let invites = invite_repo.get_by_org(org_id).await.unwrap();
        assert_eq!(invites.len(), 0);
    }
}
