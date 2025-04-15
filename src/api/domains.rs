use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiDomain, ApiUser, DomainRepository, NewDomain, OrganizationId, ProjectId},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;

fn has_write_access(
    org: OrganizationId,
    _proj: Option<ProjectId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

#[derive(Debug, Deserialize)]
pub struct DomainPath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
}

pub async fn create_domain(
    State(repo): State<DomainRepository>,
    user: ApiUser,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
    Json(new): Json<NewDomain>,
) -> ApiResult<ApiDomain> {
    has_write_access(org_id, project_id, &user)?;

    Ok(Json(repo.create(new, org_id, project_id).await?.into()))
}
