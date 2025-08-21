use crate::{
    api::{
        RemailsConfig,
        error::{ApiError, ApiResult},
    },
    handler::dns::{DnsResolver, DomainVerificationStatus},
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
use serde::Deserialize;
use tracing::{debug, info};

fn has_write_access(
    org: &OrganizationId,
    _proj: Option<&ProjectId>,
    _domain: Option<&DomainId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.is_org_admin(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

fn has_read_access(
    org: &OrganizationId,
    proj: Option<&ProjectId>,
    domain: Option<&DomainId>,
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
    State((repo, resolver, config)): State<(DomainRepository, DnsResolver, RemailsConfig)>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
    Json(new): Json<NewDomain>,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(&org_id, project_id.as_ref(), None, &user)?;

    let domain = repo.create(new, org_id, project_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    let domain = ApiDomain::verified(domain, status);

    info!(
        user_id = user.id().to_string(),
        domain_id = domain.id().to_string(),
        parent_id = ?domain.parent_id(),
        domain = domain.domain(),
        "created domain");

    Ok((StatusCode::CREATED, Json(domain)))
}

pub async fn list_domains(
    State((repo, resolver, config)): State<(DomainRepository, DnsResolver, RemailsConfig)>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
) -> ApiResult<Vec<ApiDomain>> {
    has_read_access(&org_id, project_id.as_ref(), None, &user)?;

    let domains = repo.list(org_id, project_id).await?;

    let mut verified_domains = Vec::with_capacity(domains.len());

    for domain in domains {
        let status = resolver.verify_domain(&domain, &config.spf_include).await?;

        verified_domains.push(ApiDomain::verified(domain, status));
    }

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        "verified & listed {} domains",
        verified_domains.len()
    );

    Ok(Json(verified_domains))
}

pub async fn get_domain(
    State((repo, resolver, config)): State<(DomainRepository, DnsResolver, RemailsConfig)>,
    user: ApiUser,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<ApiDomain> {
    has_read_access(&org_id, project_id.as_ref(), Some(&domain_id), &user)?;

    let domain = repo.get(org_id, project_id, domain_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    let domain = ApiDomain::verified(domain, status);

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
    has_write_access(&org_id, project_id.as_ref(), Some(&domain_id), &user)?;

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

pub(super) async fn verify_domain(
    State((repo, resolver, config)): State<(DomainRepository, DnsResolver, RemailsConfig)>,
    user: ApiUser,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<DomainVerificationStatus> {
    has_write_access(&org_id, project_id.as_ref(), Some(&domain_id), &user)?;

    let domain = repo.get(org_id, project_id, domain_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    Ok(Json(status))
}
