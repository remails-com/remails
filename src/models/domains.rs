use crate::{
    handler::dns::{DnsResolver, DomainVerificationStatus},
    models::{Error, OrganizationId, ProjectId},
};
use aws_lc_rs::{encoding::AsDer, rsa::KeySize, signature::KeyPair};
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use futures::StreamExt;
use garde::Validate;
use mail_auth::common::{crypto::Algorithm, headers::Writable};
use mail_send::mail_auth::common::crypto as mail_auth_crypto;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, query};
use std::{
    fmt::{Debug, Formatter},
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer;
use tracing::{error, info, trace};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    sqlx::Type,
    FromStr,
    ToSchema,
    IntoParams,
)]
#[sqlx(transparent)]
#[into_params(names("domain_id"))]
pub struct DomainId(Uuid);

#[derive(sqlx::Type, Serialize, Deserialize, Debug, ToSchema)]
#[sqlx(type_name = "dkim_key_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DkimKeyType {
    RsaSha256,
    Ed25519,
}

pub enum DkimKey {
    Ed25519(aws_lc_rs::signature::Ed25519KeyPair),
    RsaSha256(aws_lc_rs::rsa::KeyPair),
}

impl DkimKey {
    pub fn pub_key(
        &self,
    ) -> Result<aws_lc_rs::encoding::PublicKeyX509Der<'_>, aws_lc_rs::error::Unspecified> {
        match self {
            DkimKey::Ed25519(k) => k.public_key().as_der(),
            DkimKey::RsaSha256(k) => k.public_key().as_der(),
        }
    }

    pub fn signing_key(&self) -> Result<MailAuthSigningKey, Error> {
        match self {
            DkimKey::Ed25519(k) => Ok(MailAuthSigningKey::Ed25519(
                mail_auth_crypto::Ed25519Key::from_pkcs8_der(k.to_pkcs8()?.as_ref())?,
            )),
            DkimKey::RsaSha256(k) => Ok(MailAuthSigningKey::RsaSha256(mail_auth_crypto::RsaKey::<
                mail_auth_crypto::Sha256,
            >::from_key_der(
                PrivatePkcs8KeyDer::from(k.as_der()?.as_ref().to_vec()).into(),
            )?)),
        }
    }

    pub fn try_from_db(kind: DkimKeyType, pkcs8_der: &[u8]) -> Result<Self, Error> {
        Ok(match kind {
            DkimKeyType::RsaSha256 => {
                DkimKey::RsaSha256(aws_lc_rs::rsa::KeyPair::from_pkcs8(pkcs8_der)?)
            }
            DkimKeyType::Ed25519 => {
                DkimKey::Ed25519(aws_lc_rs::signature::Ed25519KeyPair::from_pkcs8(pkcs8_der)?)
            }
        })
    }
}

pub enum MailAuthSigningKey {
    Ed25519(mail_auth_crypto::Ed25519Key),
    RsaSha256(mail_auth_crypto::RsaKey<mail_auth_crypto::Sha256>),
}

impl mail_auth_crypto::SigningKey for MailAuthSigningKey {
    type Hasher = mail_auth_crypto::Sha256;

    fn sign(&self, input: impl Writable) -> mail_auth::Result<Vec<u8>> {
        match self {
            MailAuthSigningKey::Ed25519(k) => k.sign(input),
            MailAuthSigningKey::RsaSha256(k) => k.sign(input),
        }
    }

    fn algorithm(&self) -> Algorithm {
        match self {
            MailAuthSigningKey::Ed25519(k) => k.algorithm(),
            MailAuthSigningKey::RsaSha256(k) => k.algorithm(),
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

#[derive(Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ApiDomain {
    id: DomainId,
    organization_id: OrganizationId,
    project_ids: Vec<ProjectId>,
    domain: String,
    dkim_key_type: DkimKeyType,
    dkim_public_key: String,
    verification_status: DomainVerificationStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ApiDomain {
    pub fn id(&self) -> DomainId {
        self.id
    }

    pub fn organization_id(&self) -> OrganizationId {
        self.organization_id
    }

    pub fn project_ids(&self) -> &[ProjectId] {
        self.project_ids.as_slice()
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }
}

#[derive(Debug)]
pub struct Domain {
    pub(crate) id: DomainId,
    organization_id: OrganizationId,
    project_ids: Vec<ProjectId>,
    pub(crate) domain: String,
    pub(crate) dkim_key: DkimKey,
    verification_status: DomainVerificationStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct PgDomain {
    id: DomainId,
    domain: String,
    organization_id: OrganizationId,
    project_ids: Vec<Uuid>,
    dkim_key_type: DkimKeyType,
    dkim_pkcs8_der: Vec<u8>,
    verification_status: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgDomain> for Domain {
    type Error = Error;

    fn try_from(pg: PgDomain) -> Result<Self, Self::Error> {
        let dkim_key = DkimKey::try_from_db(pg.dkim_key_type, &pg.dkim_pkcs8_der)?;

        Ok(Self {
            id: pg.id,
            organization_id: pg.organization_id,
            project_ids: pg.project_ids.into_iter().map(Into::into).collect(),
            domain: pg.domain,
            dkim_key,
            verification_status: serde_json::from_value(pg.verification_status)?,
            created_at: pg.created_at,
            updated_at: pg.updated_at,
        })
    }
}

impl From<Domain> for ApiDomain {
    fn from(d: Domain) -> Self {
        let dkim_key_type = match d.dkim_key {
            DkimKey::Ed25519(_) => DkimKeyType::Ed25519,
            DkimKey::RsaSha256(_) => DkimKeyType::RsaSha256,
        };

        Self {
            id: d.id,
            organization_id: d.organization_id,
            project_ids: d.project_ids,
            domain: d.domain,
            dkim_key_type,
            dkim_public_key: Base64::encode_string(d.dkim_key.pub_key().expect("As we generate the keys ourselves, we should never run into a marshalling problem").as_ref()),
            verification_status: d.verification_status,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[cfg_attr(test, derive(Serialize))]
pub struct NewDomain {
    #[garde(length(min = 3, max = 253))]
    #[schema(min_length = 3, max_length = 253)]
    pub domain: String,
    #[garde(skip)]
    pub project_ids: Vec<ProjectId>,
    #[garde(skip)]
    pub dkim_key_type: DkimKeyType,
}

#[derive(Clone)]
pub struct DomainRepository {
    pool: sqlx::PgPool,
    resolver: DnsResolver,
}

impl DomainRepository {
    pub fn new(pool: sqlx::PgPool, resolver: DnsResolver) -> Self {
        Self { pool, resolver }
    }

    pub async fn create(&self, new: NewDomain, org_id: OrganizationId) -> Result<Domain, Error> {
        let (sk_bytes, pk_bytes) = match new.dkim_key_type {
            DkimKeyType::RsaSha256 => {
                let key = aws_lc_rs::rsa::KeyPair::generate(KeySize::Rsa2048)?;
                (key.as_der()?, key.public_key().as_ref().to_vec())
            }
            DkimKeyType::Ed25519 => {
                let key = aws_lc_rs::signature::Ed25519KeyPair::generate()?;
                (key.as_der()?, key.public_key().as_ref().to_vec())
            }
        };

        let verification_status = self.resolver.verify_domain(&new.domain, &pk_bytes).await?;

        let mut tx = self.pool.begin().await?;

        let id: DomainId = sqlx::query_scalar!(
            r#"
            INSERT INTO domains (id, domain, organization_id, dkim_key_type, dkim_pkcs8_der, last_verification_time, verification_status)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            new.domain,
            *org_id,
            new.dkim_key_type as DkimKeyType,
            sk_bytes.as_ref(),
            verification_status.timestamp(),
            serde_json::to_value(verification_status)?,
        ).fetch_one(&mut *tx).await?.into();

        Self::attach_projects(&mut tx, org_id, id, &new.project_ids).await?;

        let domain = Self::get_one(&mut tx, org_id, id).await?;

        tx.commit().await?;

        Ok(domain)
    }

    async fn get_one(
        tx: &mut PgConnection,
        org_id: OrganizationId,
        domain_id: DomainId,
    ) -> Result<Domain, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT d.id,
                   d.domain,
                   d.organization_id,
                   COALESCE(
                       array_agg(dp.project_id) FILTER (WHERE dp.project_id IS NOT NULL),
                       '{}'
                   ) AS "project_ids!",
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der,
                   d.verification_status,
                   d.created_at,
                   d.updated_at
            FROM domains d
            LEFT JOIN domains_projects dp ON d.id = dp.domain_id
            WHERE d.id = $2 AND d.organization_id = $1
            GROUP BY d.id
            "#,
            *org_id,
            *domain_id,
        )
        .fetch_one(tx)
        .await?
        .try_into()
    }

    pub async fn get(&self, org_id: OrganizationId, domain_id: DomainId) -> Result<Domain, Error> {
        Self::get_one(&mut *self.pool.acquire().await?, org_id, domain_id).await
    }

    pub async fn verify(
        &self,
        org_id: OrganizationId,
        domain_id: DomainId,
    ) -> Result<DomainVerificationStatus, Error> {
        let row = sqlx::query!(
            r#"
            SELECT
                   d.domain,
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der
            FROM domains d
            WHERE d.id = $2 AND d.organization_id = $1
            "#,
            *org_id,
            *domain_id
        )
        .fetch_one(&self.pool)
        .await?;

        let sk = DkimKey::try_from_db(row.dkim_key_type, &row.dkim_pkcs8_der)?;

        let verification_status = self
            .resolver
            .verify_domain(&row.domain, sk.pub_key()?.as_ref())
            .await?;

        self.store_verification_status(&domain_id, &verification_status)
            .await?;

        Ok(verification_status)
    }

    pub async fn store_verification_status(
        &self,
        domain_id: &DomainId,
        verification_status: &DomainVerificationStatus,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE domains SET verification_status = $3, last_verification_time = $2 WHERE id = $1
            "#,
            **domain_id,
            verification_status.timestamp(),
            serde_json::to_value(verification_status)?,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn verify_all(&self) -> Result<(), Error> {
        let domains = query!(
            r#"
            SELECT id, domain, dkim_key_type AS "kind:DkimKeyType", dkim_pkcs8_der
            FROM domains
            WHERE last_verification_time < now() - '20 hours'::interval
            "#
        )
        .fetch(&self.pool);

        let mut error_count = AtomicUsize::new(0);
        let mut success_count = AtomicUsize::new(0);

        domains
            .for_each_concurrent(None, async |res| match res {
                Ok(domain) => {
                    let pk = match DkimKey::try_from_db(domain.kind, &domain.dkim_pkcs8_der) {
                        Err(err) => {
                            error!(
                                domain_id = domain.id.to_string(),
                                domain = domain.domain,
                                "Encountered error while updating domain verification status: {err}"
                            );
                            error_count.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                        Ok(pk) => pk,
                    };

                    let verification = match self
                        .resolver
                        .verify_domain(
                            &domain.domain,
                            pk.pub_key()
                                .expect("We only generate the key internally, so they should work")
                                .as_ref(),
                        )
                        .await
                    {
                        Ok(v) => v,
                        Err(err) => {
                            error!(
                                domain_id = domain.id.to_string(),
                                domain = domain.domain,
                                "Encountered error while updating domain verification status: {err}"
                            );
                            error_count.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                    };

                    match self
                        .store_verification_status(&domain.id.into(), &verification)
                        .await
                    {
                        Ok(()) => {
                            trace!(
                                domain_id = domain.id.to_string(),
                                domain = domain.domain,
                                "Updated verification status of domain"
                            );
                            success_count.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(err) => {
                            error!(
                                domain_id = domain.id.to_string(),
                                domain = domain.domain,
                                "Encountered error while updating domain verification status: {err}"
                            );
                            error_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(err) => {
                    error!("Encountered error while updating domain verification status: {err}");
                    error_count.fetch_add(1, Ordering::Relaxed);
                }
            })
            .await;

        info!(
            "Updated {} DNS verifications ({} failed)",
            success_count.get_mut(),
            error_count.get_mut()
        );

        if error_count.get_mut() > &mut 0 {
            Err(Error::Internal(
                "Did not verify all DNS records".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Verify that an organization has access to the domain ID and the project IDs
    async fn check_domain_and_projects(
        tx: &mut PgConnection,
        org_id: OrganizationId,
        domain_id: DomainId,
        project_ids: &[Uuid],
    ) -> Result<(), Error> {
        let valid = sqlx::query_scalar!(
            r#"
            SELECT (
                EXISTS (SELECT 1 FROM domains WHERE organization_id = $1 AND id = $2)
                AND (
                    SELECT COUNT(*) = $4 -- all projects are valid
                    FROM projects
                    WHERE organization_id = $1 AND id = ANY($3)
               )
            ) as "valid!"
            "#,
            *org_id,
            *domain_id,
            &project_ids,
            project_ids.len() as i64
        )
        .fetch_one(&mut *tx)
        .await?;

        if !valid {
            return Err(Error::BadRequest(
                "Organization has no access to the domain or one of the projects".to_string(),
            ));
        }

        Ok(())
    }

    async fn attach_projects(
        tx: &mut PgConnection,
        org_id: OrganizationId,
        domain_id: DomainId,
        attached_projects: &[ProjectId],
    ) -> Result<Domain, Error> {
        let project_ids = attached_projects.iter().map(|p| **p).collect::<Vec<_>>();

        Self::check_domain_and_projects(tx, org_id, domain_id, &project_ids).await?;

        sqlx::query!(
            r#"
            INSERT INTO domains_projects (domain_id, project_id)
            SELECT $1, unnest($2::uuid[])
            "#,
            *domain_id,
            &project_ids
        )
        .execute(&mut *tx)
        .await?;

        Self::get_one(tx, org_id, domain_id).await
    }

    pub async fn update(
        &self,
        org_id: OrganizationId,
        domain_id: DomainId,
        attached_projects: &[ProjectId],
    ) -> Result<Domain, Error> {
        let mut tx = self.pool.begin().await?;

        // remove previously attached projects
        sqlx::query!(
            r#"
            DELETE FROM domains_projects
            WHERE domain_id = $1
            "#,
            *domain_id
        )
        .execute(&mut *tx)
        .await?;

        let domain = Self::attach_projects(&mut tx, org_id, domain_id, attached_projects).await?;

        tx.commit().await?;

        Ok(domain)
    }

    pub async fn list(&self, org_id: OrganizationId) -> Result<Vec<Domain>, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT d.id,
                   d.domain,
                   d.organization_id,
                   COALESCE(
                       array_agg(dp.project_id) FILTER (WHERE dp.project_id IS NOT NULL),
                       '{}'
                   ) AS "project_ids!",
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der,
                   d.verification_status,
                   d.created_at,
                   d.updated_at
            FROM domains d
            LEFT JOIN domains_projects dp ON d.id = dp.domain_id
            WHERE d.organization_id = $1
            GROUP BY d.id
            ORDER BY d.updated_at DESC
            "#,
            *org_id,
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
        domain_id: DomainId,
    ) -> Result<DomainId, Error> {
        let id = sqlx::query_scalar!(
            r#"
            DELETE
            FROM domains
            WHERE id = $2 AND organization_id = $1
            RETURNING domains.id
            "#,
            *org_id,
            *domain_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DomainId(id))
    }

    /// Look up a domain name in the database for a specific project, returning a domain that is attached to that project,
    /// or `None` if no matching domain was found
    ///
    /// The domain is allowed to be a sub-domain of the domain in the database
    ///
    /// In case multiple domains match, we will pick the most specific domain
    pub async fn lookup_domain_name(
        &self,
        domain: &str,
        project_id: ProjectId,
    ) -> Result<Option<Domain>, Error> {
        sqlx::query_as!(
            PgDomain,
            r#"
            SELECT d.id,
                   d.domain,
                   d.organization_id,
                   COALESCE(
                       array_agg(dp.project_id) FILTER (WHERE dp.project_id IS NOT NULL),
                       '{}'
                   ) AS "project_ids!",
                   d.dkim_key_type as "dkim_key_type: DkimKeyType",
                   d.dkim_pkcs8_der,
                   d.verification_status,
                   d.created_at,
                   d.updated_at
            FROM domains_projects dp
            LEFT JOIN domains d ON dp.domain_id = d.id
            WHERE dp.project_id = $1 AND $2 SIMILAR TO '(%.)?' || d.domain
            GROUP BY d.id
            ORDER BY char_length(d.domain) DESC
            LIMIT 1
            "#,
            *project_id,
            domain
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|domain| domain.try_into())
        .transpose()
    }
}

#[cfg(test)]
mod test {
    use crate::test::TestProjects;

    use super::*;
    use sqlx::PgPool;
    use std::{ops::Sub, str::FromStr};

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "api_users",
            "org_domains",
            "proj_domains",
        )
    ))]
    async fn check_project_for_domain(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let org_1_domain_1 = "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap();
        let org_1_subdomain_1 = "db61e35e-fe1b-46ff-aae2-070d80079626".parse().unwrap();
        let org_1_project_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let org_1_project_2 = "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap();
        let org_1_project_1_domain_1 = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap();
        let org_1_project_1_subdomain_2 = "8ef2c61e-8dd2-45a3-8b64-ef5031a9d05a".parse().unwrap();

        let valid_project_domain = repo
            .lookup_domain_name("test-org-1-project-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(valid_project_domain.id, org_1_project_1_domain_1);

        let valid_org_domain = repo
            .lookup_domain_name("test-org-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(valid_org_domain.id, org_1_domain_1);

        let domain_from_sibling_project = repo
            .lookup_domain_name("test-org-1-project-2.com", org_1_project_1)
            .await
            .unwrap();
        assert!(domain_from_sibling_project.is_none());

        let domain_from_different_org = repo
            .lookup_domain_name("test-org-2.com", org_1_project_1)
            .await
            .unwrap();
        assert!(domain_from_different_org.is_none());

        let domain_from_different_org_proj = repo
            .lookup_domain_name("test-org-2-project-1.com", org_1_project_1)
            .await
            .unwrap();
        assert!(domain_from_different_org_proj.is_none());

        let project_from_same_org = repo
            .lookup_domain_name("test-org-1.com", org_1_project_2)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project_from_same_org.id, org_1_domain_1);

        let domain_from_different_project = repo
            .lookup_domain_name("test-org-1-project-1.com", org_1_project_2)
            .await
            .unwrap();
        assert!(domain_from_different_project.is_none());

        let valid_subdomain = repo
            .lookup_domain_name("asdf.test-org-1-project-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(valid_subdomain.id, org_1_project_1_domain_1);

        let double_valid_subdomain = repo
            .lookup_domain_name("remails.asdf.test-org-1-project-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(double_valid_subdomain.id, org_1_project_1_domain_1);

        let invalid_subdomain = repo
            .lookup_domain_name("asdftest-org-1-project-1.com", org_1_project_1)
            .await
            .unwrap();
        assert!(invalid_subdomain.is_none());

        let invalid_postfix = repo
            .lookup_domain_name("test-org-1-project-1.comasdf", org_1_project_1)
            .await
            .unwrap();
        assert!(invalid_postfix.is_none());

        let project_does_not_exist = repo
            .lookup_domain_name(
                "test-org-1-project-1.com",
                "00000000-0000-4000-0000-000000000000".parse().unwrap(),
            )
            .await
            .unwrap();
        assert!(project_does_not_exist.is_none());

        let subdomain_takes_preference_org = repo
            .lookup_domain_name("subdomain.test-org-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(subdomain_takes_preference_org.id, org_1_subdomain_1);

        let subdomain_takes_preference_proj = repo
            .lookup_domain_name("subdomain2.test-org-1.com", org_1_project_1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            subdomain_takes_preference_proj.id,
            org_1_project_1_subdomain_2
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "api_users",
            "org_domains",
            "proj_domains"
        )
    ))]
    async fn create_org_does_not_match_proj(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));
        let org_1 = TestProjects::Org1Project1.org_id();
        let proj_1_org_2 = TestProjects::Org2Project1.project_id();

        let nr_of_domains = repo.list(org_1).await.unwrap().len();

        let bad_request = repo
            .create(
                NewDomain {
                    domain: "test-domain.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                    project_ids: vec![proj_1_org_2],
                },
                org_1,
            )
            .await
            .unwrap_err();
        assert!(matches!(bad_request, Error::BadRequest(_)));

        assert_eq!(
            repo.list(org_1).await.unwrap().len(),
            nr_of_domains,
            "no new domain should be created"
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn create_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));
        let (org_1, proj_1) = TestProjects::Org1Project1.get_ids();
        let proj_2 = TestProjects::Org1Project2.project_id();

        let domain = repo
            .create(
                NewDomain {
                    domain: "test-domain1.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                    project_ids: vec![proj_1],
                },
                org_1,
            )
            .await
            .unwrap();
        assert_eq!(domain.domain, "test-domain1.com");
        assert_eq!(domain.organization_id, org_1);
        assert_eq!(domain.project_ids, vec![proj_1]);

        let domain = repo
            .create(
                NewDomain {
                    domain: "test-domain2.com".to_string(),
                    dkim_key_type: DkimKeyType::Ed25519,
                    project_ids: vec![],
                },
                org_1,
            )
            .await
            .unwrap();
        assert_eq!(domain.domain, "test-domain2.com");
        assert_eq!(domain.organization_id, org_1);
        assert!(domain.project_ids.is_empty());

        let domain = repo
            .create(
                NewDomain {
                    domain: "test-domain3.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                    project_ids: vec![proj_1, proj_2],
                },
                org_1,
            )
            .await
            .unwrap();
        assert_eq!(domain.domain, "test-domain3.com");
        assert_eq!(domain.organization_id, org_1);
        assert_eq!(domain.project_ids, vec![proj_1, proj_2]);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn create_conflicting_domain(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let conflict = repo
            .create(
                NewDomain {
                    domain: "test-org-2-project-1.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                    // Project 1 Organization 1
                    project_ids: vec!["3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()],
                },
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(conflict, Error::Conflict))
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "proj_domains")
    ))]
    async fn update_happy_path(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));
        let (org_1, proj_1) = TestProjects::Org1Project1.get_ids();
        let proj_2 = TestProjects::Org1Project2.project_id();
        let domain_id = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap();

        // attach an extra project
        let domain = repo
            .update(org_1, domain_id, &[proj_1, proj_2])
            .await
            .unwrap();
        assert_eq!(domain.project_ids, vec![proj_1, proj_2]);

        // remove attached projects
        let domain = repo.update(org_1, domain_id, &[]).await.unwrap();
        assert!(domain.project_ids.is_empty());
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "proj_domains")
    ))]
    async fn update_wrong_org(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));
        let (org_1, proj_1) = TestProjects::Org1Project1.get_ids();
        let proj_1_org_2 = TestProjects::Org2Project1.project_id();
        let domain_org_1 = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap();
        let domain_org_2 = "ae5ff990-d2c3-4368-a58f-003581705086".parse().unwrap();

        // can't attach project from other organization
        assert!(matches!(
            repo.update(org_1, domain_org_1, &[proj_1_org_2])
                .await
                .unwrap_err(),
            Error::BadRequest(_)
        ));

        // can't attach project to domain from other organization
        assert!(matches!(
            repo.update(org_1, domain_org_2, &[proj_1])
                .await
                .unwrap_err(),
            Error::BadRequest(_)
        ));

        // can't remove projects from another organization's domain
        assert!(matches!(
            repo.update(org_1, domain_org_2, &[]).await.unwrap_err(),
            Error::BadRequest(_)
        ));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn get_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domain = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
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
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain.domain, "test-org-1.com")
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn list_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domains = repo
            .list(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(domains.len(), 5);
        assert!(domains.iter().any(|d| d.domain == "test-org-1.com"));
        assert!(
            domains
                .iter()
                .any(|d| d.domain == "subdomain.test-org-1.com")
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn remove_with_org_id_that_does_not_match(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domain1 = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
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
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn remove_with_org_id_that_does_not_match_proj_id(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domain1 = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
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
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn remove_happy_flow(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domain_proj = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain_proj.domain, "test-org-1-project-1.com");

        repo.remove(
            // test org 1
            "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            // test-org-1-project-1.com
            "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
        )
        .await
        .unwrap();

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // test-org-1-project-1.com
                "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(not_found, Error::NotFound(_)));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains")
    ))]
    async fn remove_happy_flow_without_project(db: PgPool) {
        let repo = DomainRepository::new(db, DnsResolver::mock("localhost", 1025));

        let domain_org = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(domain_org.domain, "test-org-1.com");

        repo.remove(
            // test org 2
            "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            // test-org-1.com
            "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
        )
        .await
        .unwrap();

        let not_found = repo
            .get(
                // test org 1
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                // test-org-1.com
                "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap(),
            )
            .await
            .unwrap_err();

        assert!(matches!(not_found, Error::NotFound(_)))
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn verify_all(db: PgPool) {
        let repo = DomainRepository::new(db.clone(), DnsResolver::mock("localhost", 1025));

        repo.verify_all().await.unwrap();

        let domains = sqlx::query!(
            r#"
            SELECT id,
                   verification_status,
                   last_verification_time
            FROM domains
            "#
        )
        .fetch_all(&db)
        .await
        .unwrap();

        for domain in domains {
            let json_timestamp: DateTime<Utc> = DateTime::from_str(
                domain
                    .verification_status
                    .get("timestamp")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            assert!(
                json_timestamp > Utc::now().sub(chrono::Duration::seconds(100)),
                "{}",
                domain.id
            );
            assert!(
                domain.last_verification_time > Utc::now().sub(chrono::Duration::seconds(100)),
                "{}",
                domain.id
            );
        }
    }
}
