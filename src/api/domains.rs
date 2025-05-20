use crate::{
    api::error::{ApiError, ApiResult},
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
