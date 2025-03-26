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
};

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
    api_user: ApiUser,
) -> ApiResult<Vec<Organization>> {
    let filter = (&api_user).into();
    let organizations = repo.list(&filter).await?;
    Ok(Json(organizations))
}

pub async fn get_organization(
    Path(id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
) -> ApiResult<Organization> {
    let filter = (&api_user).into();
    let organization = repo
        .get_by_id(id, &filter)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(organization))
}

pub async fn create_organization(
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
    Json(new): Json<NewOrganization>,
) -> ApiResult<Organization> {
    let org = repo.create(new).await?;
    repo.add_user(org.id, *api_user.id()).await?;
    Ok(Json(org))
}

pub async fn remove_organization(
    Path(id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
) -> Result<(), ApiError> {
    let filter = (&api_user).into();
    repo.remove(id, &filter).await?;
    Ok(())
}
