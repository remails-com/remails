use crate::{
    api::{
        auth::ApiUser,
        error::{ApiError, ApiResult},
    },
    models::{NewOrganization, Organization, OrganizationFilter, OrganizationRepository},
};
use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

impl TryFrom<&ApiUser> for OrganizationFilter {
    type Error = ApiError;

    fn try_from(user: &ApiUser) -> Result<Self, Self::Error> {
        if user.is_admin() {
            Ok(OrganizationFilter::default())
        } else if let Some(user_id) = user.get_user_id() {
            Ok(OrganizationFilter {
                api_user_id: Some(user_id),
            })
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

pub async fn list_organizations(
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<Organization>> {
    let filter = (&api_user).try_into()?;
    let organizations = repo.list(&filter).await?;
    Ok(Json(organizations))
}

pub async fn get_organization(
    Path(id): Path<Uuid>,
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
) -> ApiResult<Organization> {
    let filter = (&api_user).try_into()?;
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
    if let Some(user_id) = api_user.get_user_id() {
        repo.add_user(org.id, user_id).await?;
    }
    Ok(Json(org))
}

pub async fn remove_organization(
    Path(id): Path<Uuid>,
    State(repo): State<OrganizationRepository>,
    api_user: ApiUser,
) -> Result<(), ApiError> {
    let filter = (&api_user).try_into()?;
    repo.remove(id, &filter).await?;
    Ok(())
}
