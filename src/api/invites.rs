use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::{TimeDelta, Utc};
use http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use crate::{
    api::error::{ApiError, ApiResult},
    models::{
        ApiInvite, ApiUser, InviteId, InviteRepository, OrganizationId, OrganizationRepository,
    },
};

fn has_read_access(user: &ApiUser, org: &OrganizationId) -> Result<(), ApiError> {
    if user.org_admin().contains(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

fn has_write_access(user: &ApiUser, org: &OrganizationId) -> Result<(), ApiError> {
    if user.org_admin().contains(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

#[derive(Debug, Deserialize)]
pub struct InvitePath {
    org_id: OrganizationId,
}

pub async fn create_invite(
    State(repo): State<InviteRepository>,
    Path(InvitePath { org_id }): Path<InvitePath>,
    user: ApiUser,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(&user, &org_id)?;

    let expires = Utc::now() + TimeDelta::days(7);
    let invite = repo.create(org_id, *user.id(), expires).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "created invite"
    );

    Ok((StatusCode::CREATED, Json(invite)))
}

pub async fn get_org_invites(
    State(repo): State<InviteRepository>,
    Path(InvitePath { org_id }): Path<InvitePath>,
    user: ApiUser,
) -> ApiResult<Vec<ApiInvite>> {
    has_read_access(&user, &org_id)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "get org invites"
    );

    let invites = repo.get_by_org(org_id).await?;
    Ok(Json(invites))
}

pub async fn get_invite(
    State(repo): State<InviteRepository>,
    Path((org_id, invite_id, password)): Path<(OrganizationId, InviteId, String)>,
    user: ApiUser,
) -> ApiResult<ApiInvite> {
    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "get invite"
    );

    let invite = repo.get_by_id(invite_id, org_id).await?;
    if !invite.verify_password(&password) {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(invite))
}

pub async fn remove_invite(
    State(repo): State<InviteRepository>,
    Path((org_id, invite_id)): Path<(OrganizationId, InviteId)>,
    user: ApiUser,
) -> ApiResult<InviteId> {
    has_write_access(&user, &org_id)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "remove invite"
    );

    let id = repo.remove_by_id(invite_id, org_id).await?;

    Ok(Json(id))
}

pub async fn accept_invite(
    State((invites, organizations)): State<(InviteRepository, OrganizationRepository)>,
    Path((org_id, invite_id, password)): Path<(OrganizationId, InviteId, String)>,
    user: ApiUser,
) -> Result<impl IntoResponse, ApiError> {
    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        invite_id = invite_id.to_string(),
        "accepting invite"
    );

    let invite = invites.get_by_id(invite_id, org_id).await?;

    if !invite.verify_password(&password) {
        return Err(ApiError::Forbidden);
    }

    if invite.is_expired() {
        return Err(ApiError::Forbidden);
    }

    organizations.add_user(org_id, *user.id()).await?;

    invites.remove_by_id(invite_id, org_id).await?;

    let Some(organization) = organizations.get_by_id(org_id).await? else {
        return Err(ApiError::Forbidden); // this shouldn't happen
    };

    Ok((StatusCode::CREATED, Json(organization)))
}
