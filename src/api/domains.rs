use crate::{
    api::error::{ApiError, ApiResult},
    models::{
        ApiDomain, ApiUser, DomainId, DomainRepository, NewDomain, OrganizationId, ProjectId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;

fn has_write_access(
    org: OrganizationId,
    _proj: Option<ProjectId>,
    _domain: Option<DomainId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
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
) -> ApiResult<ApiDomain> {
    has_write_access(org_id, project_id, None, &user)?;

    Ok(Json(repo.create(new, org_id, project_id).await?.into()))
}

pub async fn list_domains(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
) -> ApiResult<Vec<ApiDomain>> {
    has_read_access(org_id, project_id, None, &user)?;

    Ok(Json(
        repo.list(org_id, project_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
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

    Ok(Json(repo.get(org_id, project_id, domain_id).await?.into()))
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

    Ok(Json(repo.remove(org_id, project_id, domain_id).await?))
}
