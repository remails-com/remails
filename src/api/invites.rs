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
    if user.is_org_admin(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

fn has_write_access(user: &ApiUser, org: &OrganizationId) -> Result<(), ApiError> {
    if user.is_org_admin(org) || user.is_super_admin() {
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

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::StatusCode;
    use sqlx::PgPool;

    use crate::{
        api::{
            tests::{TestServer, deserialize_body},
            whoami::WhoamiResponse,
        },
        models::{CreatedInviteWithPassword, OrgRole, Organization, Role},
    };

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_create_and_use_invite(pool: PgPool) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let user_b = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let user_c = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let _org_2 = "5d55aec5-136a-407c-952f-5348d4398204";
        let mut server = TestServer::new(pool.clone(), user_a).await;

        // start with no invites
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invites: Vec<ApiInvite> = deserialize_body(response.into_body()).await;
        assert_eq!(invites.len(), 0);

        // create a new invite
        let response = server
            .post(format!("/api/invite/{org_1}"), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_invite: CreatedInviteWithPassword =
            deserialize_body(response.into_body()).await;
        assert_eq!(created_invite.organization_id().to_string(), org_1);
        assert_eq!(*created_invite.created_by(), user_a);

        // new invite created
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invites: Vec<ApiInvite> = deserialize_body(response.into_body()).await;
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].id(), created_invite.id());
        let invite_endpoint = format!(
            "/api/invite/{org_1}/{}/{}",
            created_invite.id(),
            created_invite.password()
        );

        // switch to other user
        server.set_user(user_b);

        // can't get all invites for other organizations
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't create invite for other organizations
        let response = server
            .post(format!("/api/invite/{org_1}"), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // get the previously create invite
        let response = server.get(invite_endpoint.clone()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invite: ApiInvite = deserialize_body(response.into_body()).await;
        assert_eq!(invite.organization_id().to_string(), org_1);
        assert_eq!(*invite.created_by(), user_a);

        // use invite
        let response = server
            .post(invite_endpoint.clone(), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id().to_string(), org_1);

        // user_b should now be an admin of org_1
        let response = server.get("/api/whoami".to_owned()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        assert_eq!(whoami.id, user_b);
        assert!(whoami.org_roles.contains(&OrgRole {
            role: Role::Admin,
            org_id: org_1.parse().unwrap()
        }));

        // now user_b can see the invites of org_1
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invites: Vec<ApiInvite> = deserialize_body(response.into_body()).await;
        assert_eq!(invites.len(), 0); // invite was removed after use

        // user_c can't get or use the same invite
        server.set_user(user_c);
        let response = server
            .post(invite_endpoint.clone(), Body::empty())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = server.get(invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
