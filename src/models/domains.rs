use crate::models::{Error, OrganizationId, ProjectId};
use aws_lc_rs::{
    encoding::AsDer,
    rsa::{KeyPair, KeySize},
};
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use mail_send::mail_auth::common::crypto::{Ed25519Key, RsaKey, Sha256};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use std::fmt::{Debug, Formatter};
use tracing::error;
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, sqlx::Type, FromStr,
)]
#[sqlx(transparent)]
pub struct DomainId(Uuid);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DomainParent {
    Organization(OrganizationId),
    Project(ProjectId),
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "dkim_key_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
enum DkimKeyType {
    RsaSha256,
    Ed25519,
}

pub enum DkimKey {
    Ed25519(Ed25519Key),
    RsaSha256(RsaKey<Sha256>),
}

impl DkimKey {
    pub fn pub_key(&self) -> Vec<u8> {
        match self {
            DkimKey::Ed25519(k) => k.public_key(),
            DkimKey::RsaSha256(k) => k.public_key(),
        }
    }
}

impl Debug for DkimKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DkimKey::Ed25519(_) => {
                write!(f, "DkimKey::Ed25519")
            }
            DkimKey::RsaSha256(_) => {
                write!(f, "DkimKey::RsaSha256")
            }
        }
    }
}

#[derive(Serialize)]
pub struct ApiDomain {
    id: DomainId,
    parent_id: DomainParent,
    domain: String,
    dkim_key_type: Option<DkimKeyType>,
    dkim_public_key: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Domain {
    id: DomainId,
    parent_id: DomainParent,
    domain: String,
    dkim_key: Option<DkimKey>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct PgDomain {
    id: DomainId,
    organization_id: Option<Uuid>,
    project_id: Option<Uuid>,
    domain: String,
    dkim_key_type: Option<DkimKeyType>,
    dkim_pkcs8_der: Option<Vec<u8>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgDomain> for Domain {
    type Error = Error;

    fn try_from(pg: PgDomain) -> Result<Self, Self::Error> {
        let parent_id = if let Some(org) = pg.organization_id {
            if pg.project_id.is_some() {
                error!("Domain has a organization and project as parent");
            }
            DomainParent::Organization(org.into())
        } else if let Some(proj) = pg.project_id {
            DomainParent::Project(proj.into())
        } else {
            error!("Domain does not have a parent");
            Err(Error::Internal("Domain does not have a parent".to_string()))?
        };

        let dkim_key = pg
            .dkim_key_type
            .zip(pg.dkim_pkcs8_der)
            .map(|(t, b)| {
                Ok::<_, Error>(match t {
                    DkimKeyType::RsaSha256 => DkimKey::RsaSha256(RsaKey::from_pkcs8_der(&b)?),
                    DkimKeyType::Ed25519 => DkimKey::Ed25519(Ed25519Key::from_pkcs8_der(&b)?),
                })
            })
            .transpose()?;

        Ok(Self {
            id: pg.id,
            parent_id,
            domain: pg.domain,
            dkim_key,
            created_at: pg.created_at,
            updated_at: pg.updated_at,
        })
    }
}

impl From<Domain> for ApiDomain {
    fn from(d: Domain) -> Self {
        let dkim_key_type = d.dkim_key.as_ref().map(|k| match k {
            DkimKey::Ed25519(_) => DkimKeyType::Ed25519,
            DkimKey::RsaSha256(_) => DkimKeyType::RsaSha256,
        });

        Self {
            id: d.id,
            parent_id: d.parent_id,
            domain: d.domain,
            dkim_key_type,
            dkim_public_key: d.dkim_key.map(|k| Base64::encode_string(&k.pub_key())),
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewDomain {
    domain: String,
    dkim_key_type: DkimKeyType,
}

pub struct DomainRepository {
    pool: sqlx::PgPool,
}

impl DomainRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        new: NewDomain,
        org_id: OrganizationId,
        proj_id: Option<ProjectId>,
    ) -> Result<Domain, Error> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT count(p.id) FROM projects p 
                               WHERE p.organization_id = $1 
                                 AND ($2::uuid IS NULL OR p.id = $2)
            "#,
            *org_id,
            proj_id.map(|p| p.as_uuid())
        )
        .fetch_one(&self.pool)
        .await?;

        if !matches!(count, Some(1)) {
            return Err(Error::BadRequest(
                "Project ID does not match organization ID".to_string(),
            ));
        }

        let parent_id = if let Some(proj_id) = proj_id {
            DomainParent::Project(proj_id)
        } else {
            DomainParent::Organization(org_id)
        };

        let key_bytes: Vec<u8> = match new.dkim_key_type {
            DkimKeyType::RsaSha256 => {
                let key_pair = KeyPair::generate(KeySize::Rsa2048)?;
                key_pair.as_der()?.as_ref().into()
            }
            DkimKeyType::Ed25519 => Ed25519Key::generate_pkcs8()?,
        };

        let (org_id, proj_id): (Option<Uuid>, Option<Uuid>) = match parent_id {
            DomainParent::Organization(org) => (Some(org.as_uuid()), None),
            DomainParent::Project(proj) => (None, Some(proj.as_uuid())),
        };

        let mut tx = self.pool.begin().await?;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO domains (id, domain, organization_id, project_id, dkim_key_type, dkim_pkcs8_der)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)
            RETURNING id 
            "#,
            new.domain,
            org_id,
            proj_id,
            new.dkim_key_type as DkimKeyType,
            key_bytes,
        ).fetch_one(&mut *tx).await?;

        let domain = Self::get_one(&mut tx, id.into()).await?;

        tx.commit().await?;

        Ok(domain)
    }

    async fn get_one(tx: &mut PgConnection, id: DomainId) -> Result<Domain, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT id,
                   domain,
                   organization_id,
                   project_id,
                   dkim_key_type as "dkim_key_type: DkimKeyType",
                   dkim_pkcs8_der,
                   created_at,
                   updated_at                
            FROM domains
            WHERE id = $1
            "#,
            *id
        )
        .fetch_one(tx)
        .await?
        .try_into()
    }

    pub async fn get(
        &self,
        org_id: OrganizationId,
        proj_id: Option<ProjectId>,
        domain_id: DomainId,
    ) -> Result<Domain, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT d.id,
                   d.domain,
                   d.organization_id,
                   d.project_id,
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der,
                   d.created_at,
                   d.updated_at                
            FROM domains d 
                LEFT JOIN projects p ON d.project_id = p.id
                LEFT JOIN organizations o ON d.organization_id = o.id 
            WHERE d.id = $3
              AND (
                  ($2::uuid IS NULL AND d.organization_id = $1) 
                      OR
                  ($2 IS NOT NULL AND d.project_id = $2 AND p.organization_id = $1)
                  )
            "#,
            *org_id,
            proj_id.map(|p| p.as_uuid()),
            *domain_id
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()
    }

    pub async fn list(
        &self,
        org_id: OrganizationId,
        proj_id: Option<ProjectId>,
    ) -> Result<Vec<Domain>, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT d.id,
                   d.domain,
                   d.organization_id,
                   d.project_id,
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der,
                   d.created_at,
                   d.updated_at
            FROM domains d 
                LEFT JOIN projects p ON p.id = d.project_id
            WHERE (d.organization_id = $1 AND $2::uuid IS NULL)
               OR ($2 IS NOT NULL
                       AND (d.project_id = $2 AND p.organization_id = $1)
                  )
            "#,
            *org_id,
            proj_id.map(|p| p.as_uuid()),
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    pub async fn remove(
        &self,
        org_id: OrganizationId,
        proj_id: Option<ProjectId>,
        domain_id: DomainId,
    ) -> Result<DomainId, Error> {
        let id = sqlx::query_scalar!(
            r#"
            DELETE
            FROM domains d
                USING projects p
            WHERE (d.project_id IS NULL OR
                   (
                       d.project_id = p.id
                           AND d.project_id = $2
                           AND p.organization_id = $1
                       )
                )
              AND (d.organization_id IS NULL OR
                   (d.organization_id = $1
                        AND $2::uuid IS NULL)
                  )
              AND (d.id = $3)
            RETURNING d.id
            "#,
            *org_id,
            proj_id.map(|p| p.as_uuid()),
            *domain_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DomainId(id))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn create_org_does_not_match_proj(db: PgPool) {
        let repo = DomainRepository::new(db);

        let bad_request = repo
            .create(
                NewDomain {
                    domain: "test-domain.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                },
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 2
                Some("70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap()),
            )
            .await
            .unwrap_err();
        assert!(matches!(bad_request, Error::BadRequest(_)))
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn create_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain = repo
            .create(
                NewDomain {
                    domain: "test-domain1.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                },
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(domain.domain, "test-domain1.com");
        assert_eq!(
            domain.parent_id,
            DomainParent::Project("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap())
        );

        let domain = repo
            .create(
                NewDomain {
                    domain: "test-domain2.com".to_string(),
                    dkim_key_type: DkimKeyType::Ed25519,
                },
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(domain.domain, "test-domain2.com");
        assert_eq!(
            domain.parent_id,
            DomainParent::Organization("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn create_conflicting_domain(db: PgPool) {
        let repo = DomainRepository::new(db);

        let conflict = repo
            .create(
                NewDomain {
                    domain: "test-org-2-project-1.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                },
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
            )
            .await
            .unwrap_err();
        assert!(matches!(conflict, Error::Conflict))
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn get_org_does_not_match_proj(db: PgPool) {
        let repo = DomainRepository::new(db);

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 2
                Some("70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap()),
                // test-org-2-project-1.com
                "ae5ff990-d2c3-4368-a58f-003581705086".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 2
                Some("70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn get_with_proj_id_from_org_domain(db: PgPool) {
        let repo = DomainRepository::new(db);

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn get_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain.domain, "test-org-1-project-1.com");

        let domain = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain.domain, "test-org-1.com")
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn list_org_does_not_match_proj(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domains = repo
            .list(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 2
                Some("70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap()),
            )
            .await
            .unwrap();
        assert!(domains.is_empty());
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn list_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domains = repo
            .list(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].domain, "test-org-1.com");

        let domains = repo
            .list(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].domain, "test-org-1-project-1.com");
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn remove_with_project_id_that_does_not_match_org_id(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain1 = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain1.domain, "test-org-1-project-1.com");

        let not_found = repo
            .remove(
                // test org 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let _still_there = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn remove_with_org_id_that_does_not_match_proj_id(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain1 = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain1.domain, "test-org-1.com");

        let not_found = repo
            .remove(
                // test org 2
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
                // Project 1 Organization 1
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(not_found, Error::NotFound(_)));

        let _still_there = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn remove_with_proj_id_from_org_domain(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain.domain, "test-org-1.com");

        let not_found = repo
            .remove(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(not_found, Error::NotFound(_)));

        let _still_there = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "domains")
    ))]
    async fn remove_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db);

        let domain_proj = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain_proj.domain, "test-org-1-project-1.com");

        repo.remove(
            // test org 1
            "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            // Project 1 Organization 1
            Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
            // test-org-1-project-1.com
            "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
        )
        .await
        .unwrap();

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // Project 1 Organization 1
                Some("3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(not_found, Error::NotFound(_)));

        let domain_org = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain_org.domain, "test-org-1.com");

        repo.remove(
            // test org 2
            "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            None,
            // test-org-1.com
            "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        )
        .await
        .unwrap();

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                None,
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(not_found, Error::NotFound(_)))
    }
}
