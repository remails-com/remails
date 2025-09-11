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
        Role,
    },
};

#[derive(Debug, Deserialize)]
pub struct InvitePath {
    org_id: OrganizationId,
}

pub async fn create_invite(
    State(repo): State<InviteRepository>,
    Path(InvitePath { org_id }): Path<InvitePath>,
    user: ApiUser,
    Json(role): Json<Role>,
) -> Result<impl IntoResponse, ApiError> {
    user.has_org_admin_access(&org_id)?;

    let expires = Utc::now() + TimeDelta::days(7);
    let invite = repo.create(org_id, role, *user.id(), expires).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        role = role.to_string(),
        "created invite"
    );

    Ok((StatusCode::CREATED, Json(invite)))
}

pub async fn get_org_invites(
    State(repo): State<InviteRepository>,
    Path(InvitePath { org_id }): Path<InvitePath>,
    user: ApiUser,
) -> ApiResult<Vec<ApiInvite>> {
    user.has_org_read_access(&org_id)?;

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
        return Err(ApiError::NotFound);
    }

    Ok(Json(invite))
}

pub async fn remove_invite(
    State(repo): State<InviteRepository>,
    Path((org_id, invite_id)): Path<(OrganizationId, InviteId)>,
    user: ApiUser,
) -> ApiResult<InviteId> {
    user.has_org_admin_access(&org_id)?;

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
        return Err(ApiError::NotFound);
    }

    if invite.is_expired() {
        return Err(ApiError::NotFound);
    }

    organizations
        .add_user(org_id, *user.id(), invite.role())
        .await?;

    invites.remove_by_id(invite_id, org_id).await?;

    let Some(organization) = organizations.get_by_id(org_id).await? else {
        tracing::error!("organization not found after accepting invite: {}", org_id);
        return Err(ApiError::NotFound); // this shouldn't happen
    };

    Ok((StatusCode::CREATED, Json(organization)))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::StatusCode;
    use sqlx::PgPool;
    use tokio_util::sync::CancellationToken;

    use crate::{
        HandlerConfig,
        api::{
            tests::{TestServer, deserialize_body, serialize_body},
            whoami::WhoamiResponse,
        },
        handler::{Handler, dns::DnsResolver},
        models::{CreatedInviteWithPassword, OrgRole, Organization, Role},
    };

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_create_and_use_invite(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let mut server = TestServer::new(pool, Some(user_1)).await;

        // start with no invites
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invites: Vec<ApiInvite> = deserialize_body(response.into_body()).await;
        assert_eq!(invites.len(), 0);

        // create a new invite
        let response = server
            .post(format!("/api/invite/{org_1}"), serialize_body(Role::Admin))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_invite: CreatedInviteWithPassword =
            deserialize_body(response.into_body()).await;
        assert_eq!(created_invite.organization_id().to_string(), org_1);
        assert_eq!(*created_invite.created_by(), user_1);

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
        server.set_user(Some(user_2));

        // can't get all invites for other organizations
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // get the previously create invite
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invite: ApiInvite = deserialize_body(response.into_body()).await;
        assert_eq!(invite.organization_id().to_string(), org_1);
        assert_eq!(*invite.created_by(), user_1);

        // accept invite
        let response = server.post(&invite_endpoint, Body::empty()).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id().to_string(), org_1);

        // user_2 should now be an admin of org_1
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_2);
        assert!(whoami.org_roles.contains(&OrgRole {
            role: Role::Admin,
            org_id: org_1.parse().unwrap()
        }));

        // now user_2 can see the invites of org_1
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let invites: Vec<ApiInvite> = deserialize_body(response.into_body()).await;
        assert_eq!(invites.len(), 0); // invite was removed after use

        // user_3 can't get or use the same invite
        server.set_user(Some(user_3));
        let response = server.post(&invite_endpoint, Body::empty()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_invite_roles(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1: OrganizationId = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let server = TestServer::new(pool.clone(), Some(user_3)).await;
        let org_repo = OrganizationRepository::new(pool);

        // admin invite
        let response = server
            .post(
                format!("/api/invite/{org_1}/32bba198-fdd8-4cb7-8b82-85857dd2527f/unsecure"),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id(), org_1);

        // user_3 should now be an admin of org_1
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_3);
        assert!(whoami.org_roles.contains(&OrgRole {
            role: Role::Admin,
            org_id: org_1,
        }));

        org_repo.remove_member(org_1, user_3).await.unwrap();

        // maintainer invite
        let response = server
            .post(
                format!("/api/invite/{org_1}/516e1804-1d4b-44d4-b4ac-9d81a6b554e7/unsecure"),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id(), org_1);

        // user_3 should now be an maintainer of org_1
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_3);
        assert!(whoami.org_roles.contains(&OrgRole {
            role: Role::Maintainer,
            org_id: org_1,
        }));

        org_repo.remove_member(org_1, user_3).await.unwrap();

        // read-only invite
        let response = server
            .post(
                format!("/api/invite/{org_1}/dbbddca4-1e50-42bb-ac6e-6e8034ba666b/unsecure"),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id(), org_1);

        // user_3 should now be an read-only in org_1
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_3);
        assert!(whoami.org_roles.contains(&OrgRole {
            role: Role::ReadOnly,
            org_id: org_1,
        }));

        org_repo.remove_member(org_1, user_3).await.unwrap();
    }

    async fn test_invites_no_access(
        server: &mut TestServer,
        read_status_code: StatusCode,
        write_status_code: StatusCode,
    ) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let active_invite: InviteId = "32bba198-fdd8-4cb7-8b82-85857dd2527f".parse().unwrap();

        // can't get all invites for other organizations
        let response = server.get(format!("/api/invite/{org_1}")).await.unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't create invite for other organizations
        let response = server
            .post(format!("/api/invite/{org_1}"), serialize_body(Role::Admin))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        let response = server
            .post(
                format!("/api/invite/{org_1}"),
                serialize_body(Role::Maintainer),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        let response = server
            .post(
                format!("/api/invite/{org_1}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't delete invite
        let response = server
            .delete(format!("/api/invite/{}/{}", org_1, active_invite))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_invites_no_access_wrong_user(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let mut server = TestServer::new(pool, Some(user_3)).await;
        test_invites_no_access(&mut server, StatusCode::FORBIDDEN, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_invites_no_access_non_admin(pool: PgPool) {
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // maintainer of org 1
        let mut server = TestServer::new(pool, Some(user_4)).await;
        test_invites_no_access(&mut server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_invites_no_access_not_logged_in(pool: PgPool) {
        let mut server = TestServer::new(pool, None).await;
        test_invites_no_access(
            &mut server,
            StatusCode::UNAUTHORIZED,
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_cannot_use_removed_invite(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let active_invite: InviteId = "32bba198-fdd8-4cb7-8b82-85857dd2527f".parse().unwrap();
        let mut server = TestServer::new(pool, None).await;

        // can get removed invite
        server.set_user(Some(user_3));
        let invite_endpoint = format!("/api/invite/{org_1}/{}/{}", active_invite, "unsecure");
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // remove invite
        server.set_user(Some(user_1));
        let response = server
            .delete(format!("/api/invite/{}/{}", org_1, active_invite))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            active_invite,
            deserialize_body::<InviteId>(response.into_body()).await
        );

        // can't get removed invite
        server.set_user(Some(user_3));
        let invite_endpoint = format!("/api/invite/{org_1}/{}/{}", active_invite, "unsecure");
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // can't accept removed invite
        let response = server.post(&invite_endpoint, Body::empty()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_wrong_invite_password(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let active_invite: InviteId = "32bba198-fdd8-4cb7-8b82-85857dd2527f".parse().unwrap();
        let server = TestServer::new(pool, Some(user_3)).await;

        let invite_endpoint = format!("/api/invite/{org_1}/{}/{}", active_invite, "wrong-password");

        // can't get invite data with wrong password
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // can't accept invite with wrong password
        let response = server.post(&invite_endpoint, Body::empty()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users", "invites")))]
    async fn test_expired_invite(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // is not in any org
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let expired_invite: InviteId = "8b01ce56-4304-47c7-b9a6-62bd1b7e8269".parse().unwrap();
        let server = TestServer::new(pool.clone(), Some(user_3)).await;

        let invite_endpoint = format!("/api/invite/{org_1}/{}/{}", expired_invite, "unsecure");

        // can get expired invite
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // can't accept expired invite
        let response = server.post(&invite_endpoint, Body::empty()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // expired invite will be removed eventually
        let config = HandlerConfig {
            allow_plain: true,
            domain: "test".to_string(),
            resolver: DnsResolver::mock("localhost", 0),
            retry_delay: chrono::Duration::minutes(5),
            max_automatic_retries: 3,
        };
        let message_handler = Handler::new(pool.clone(), config.into(), CancellationToken::new());
        message_handler.periodic_clean_up().await.unwrap();

        // cannot get automatically removed expired invite
        let response = server.get(&invite_endpoint).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
