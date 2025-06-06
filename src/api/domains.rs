use crate::{
    api::error::{ApiError, ApiResult},
    dkim::PrivateKey,
    handler::dns::DnsResolver,
    models::{
        ApiDomain, ApiUser, DomainId, DomainRepository, NewDomain, OrganizationId, ProjectId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

fn has_write_access(
    org: OrganizationId,
    _proj: Option<ProjectId>,
    _domain: Option<DomainId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().contains(&org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

fn has_read_access(
    org: OrganizationId,
    proj: Option<ProjectId>,
    domain: Option<DomainId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, proj, domain, user)
}

#[derive(Debug, Deserialize)]
pub struct DomainPath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
}

#[derive(Debug, Deserialize)]
pub struct SpecificDomainPath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
    domain_id: DomainId,
}

pub async fn create_domain(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
    Json(new): Json<NewDomain>,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(org_id, project_id, None, &user)?;

    let domain: ApiDomain = repo.create(new, org_id, project_id).await?.into();

    info!(
        user_id = user.id().to_string(),
        domain_id = domain.id().to_string(),
        parent_id = ?domain.parent_id(),
        domain = domain.domain(),
        "created domain");

    Ok((StatusCode::CREATED, Json(domain)))
}

pub async fn list_domains(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
) -> ApiResult<Vec<ApiDomain>> {
    has_read_access(org_id, project_id, None, &user)?;

    let domains = repo
        .list(org_id, project_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect::<Vec<ApiDomain>>();

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        "listed {} domains",
        domains.len()
    );

    Ok(Json(domains))
}

pub async fn get_domain(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<ApiDomain> {
    has_read_access(org_id, project_id, Some(domain_id), &user)?;

    let domain: ApiDomain = repo.get(org_id, project_id, domain_id).await?.into();

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        domain_id = domain_id.to_string(),
        domain = domain.domain(),
        "retrieved domain",
    );

    Ok(Json(domain))
}

pub async fn delete_domain(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<DomainId> {
    has_write_access(org_id, project_id, Some(domain_id), &user)?;

    let domain_id = repo.remove(org_id, project_id, domain_id).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        domain_id = domain_id.to_string(),
        "deleted domain",
    );

    Ok(Json(domain_id))
}

#[derive(Deserialize, Serialize)]
pub(crate) enum VerifyResultStatus {
    Success,
    Warning,
    Error,
}

#[derive(Deserialize, Serialize)]
pub struct VerifyResult {
    pub(crate) status: VerifyResultStatus,
    pub(crate) reason: String,
    pub(crate) value: Option<String>,
}

impl VerifyResult {
    pub fn error(reason: impl Into<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Error,
            reason: reason.into(),
            value: None,
        }
    }
    pub fn warning(reason: impl Into<String>, value: Option<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Warning,
            reason: reason.into(),
            value,
        }
    }
    pub fn success(reason: impl Into<String>) -> Self {
        VerifyResult {
            status: VerifyResultStatus::Success,
            reason: reason.into(),
            value: None,
        }
    }
}

impl From<Result<&'static str, &'static str>> for VerifyResult {
    fn from(value: Result<&'static str, &'static str>) -> Self {
        VerifyResult {
            status: value
                .map(|_| VerifyResultStatus::Success)
                .unwrap_or(VerifyResultStatus::Error),
            reason: value.unwrap_or_else(|e| e).to_string(),
            value: None,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct DomainVerificationResult {
    dkim: VerifyResult,
    spf: VerifyResult,
    dmarc: VerifyResult,
}

pub(super) async fn verify_domain(
    State((repo, resolver)): State<(DomainRepository, DnsResolver)>,
    user: ApiUser,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<DomainVerificationResult> {
    has_write_access(org_id, project_id, Some(domain_id), &user)?;

    let domain = repo.get(org_id, project_id, domain_id).await?;

    let domain_name = domain.domain.clone();
    let db_key = PrivateKey::new(&domain, "remails")?;

    Ok(Json(DomainVerificationResult {
        dkim: resolver
            .verify_dkim(&domain_name, db_key.public_key())
            .await
            .into(),
        spf: resolver.verify_spf(&domain_name).await,
        dmarc: resolver.verify_dmarc(&domain_name).await,
    }))
}
