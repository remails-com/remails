use crate::{
    api::error::{ApiError, ApiResult},
    models::{
        ApiUser, NewOrganization, Organization, OrganizationFilter, OrganizationId,
        OrganizationRepository,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

impl From<&ApiUser> for OrganizationFilter {
    fn from(user: &ApiUser) -> Self {
        if user.is_super_admin() {
            OrganizationFilter::default()
        } else {
            OrganizationFilter {
                orgs: Some(user.org_admin()),
            }
        }
    }
}

pub async fn list_organizations(
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
) -> ApiResult<Vec<Organization>> {
    let filter = (&user).into();
    let organizations = repo.list(&filter).await?;

    debug!(
        user_id = user.id().to_string(),
        "listed {} organizations",
        organizations.len()
    );

    Ok(Json(organizations))
}

pub async fn get_organization(
    Path(id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
) -> ApiResult<Organization> {
    let filter = (&user).into();
    let organization = repo
        .get_by_id(id, &filter)
        .await?
        .ok_or(ApiError::NotFound)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = id.to_string(),
        organization_name = organization.name,
        "retrieved organization",
    );

    Ok(Json(organization))
}

pub async fn create_organization(
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
    Json(new): Json<NewOrganization>,
) -> Result<impl IntoResponse, ApiError> {
    let org = repo.create(new).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.id().to_string(),
        organization_name = org.name,
        "created organization"
    );

    repo.add_user(org.id(), *user.id()).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.id().to_string(),
        organization_name = org.name,
        "added user as organization admin"
    );

    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn remove_organization(
    Path(id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
) -> ApiResult<OrganizationId> {
    let filter = (&user).into();

    let organization_id = repo.remove(id, &filter).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = organization_id.to_string(),
        "deleted organization",
    );

    Ok(Json(organization_id))
}
