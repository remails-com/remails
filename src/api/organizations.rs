use crate::{
    api::{
        auth::Authenticated,
        error::{ApiError, ApiResult},
    },
    models::{
        ApiUser, ApiUserId, NewOrganization, OrgBlockStatus, Organization, OrganizationId,
        OrganizationMember, OrganizationRepository, Role,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

pub async fn list_organizations(
    State(repo): State<OrganizationRepository>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Vec<Organization>> {
    let filter = user.viewable_organizations_filter();
    let organizations = repo.list(filter).await?;

    debug!(
        user_id = user.log_id(),
        "listed {} organizations",
        organizations.len()
    );

    Ok(Json(organizations))
}

pub async fn get_organization(
    Path(org_id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Organization> {
    user.has_org_read_access(&org_id)?;

    let organization = repo.get_by_id(org_id).await?.ok_or(ApiError::NotFound)?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        organization_name = organization.name,
        "retrieved organization",
    );

    Ok(Json(organization))
}

pub async fn create_organization(
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users are allowed to create organizations
    Json(new): Json<NewOrganization>,
) -> Result<impl IntoResponse, ApiError> {
    let org = repo.create(new).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.id().to_string(),
        organization_name = org.name,
        "created organization"
    );

    let role = Role::ReadOnly;
    repo.add_member(org.id(), *user.id(), role).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org.id().to_string(),
        organization_name = org.name,
        role = role.to_string(),
        "added user to organization"
    );

    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn remove_organization(
    Path(org_id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users are allowed to remove organizations
) -> ApiResult<OrganizationId> {
    user.has_org_admin_access(&org_id)?;

    let organization_id = repo.remove(org_id).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = organization_id.to_string(),
        "deleted organization",
    );

    Ok(Json(organization_id))
}

pub async fn update_organization(
    Path(org_id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users are allowed to update organizations
    Json(update): Json<NewOrganization>,
) -> ApiResult<Organization> {
    user.has_org_admin_access(&org_id)?;

    let organization = repo.update(org_id, update).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = organization.id().to_string(),
        "updated organization",
    );

    Ok(Json(organization))
}

pub async fn list_members(
    Path(org_id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Vec<OrganizationMember>> {
    user.has_org_read_access(&org_id)?;

    let members = repo.list_members(org_id).await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        "listed {} members",
        members.len()
    );

    Ok(Json(members))
}

async fn prevent_last_remaining_admin(
    repo: &OrganizationRepository,
    org_id: &OrganizationId,
    user_id: &ApiUserId,
) -> Result<(), ApiError> {
    let members = repo.list_members(*org_id).await?;

    let any_other_admins = members
        .iter()
        .any(|m| *m.role() == Role::Admin && *m.user_id() != *user_id);

    any_other_admins
        .then_some(())
        .ok_or(ApiError::PreconditionFailed(
            "At least one admin must remain in the organization",
        ))
}

pub async fn remove_member(
    Path((org_id, user_id)): Path<(OrganizationId, ApiUserId)>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users are allowed to remove members
) -> ApiResult<()> {
    // admins can remove any member, non-admin users can remove themselves
    user.has_org_admin_access(&org_id)
        .or(user.has_org_read_access(&org_id).and(
            (user_id == *user.id())
                .then_some(())
                .ok_or(ApiError::Forbidden),
        ))?;

    if user.has_org_admin_access(&org_id).is_ok() && *user.id() == user_id {
        prevent_last_remaining_admin(&repo, &org_id, &user_id).await?;
    }

    repo.remove_member(org_id, user_id).await?;

    info!(
        removed_user_id = user_id.to_string(),
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "removed user from organization",
    );

    Ok(Json(()))
}

pub async fn update_member_role(
    Path((org_id, user_id)): Path<(OrganizationId, ApiUserId)>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users are allowed to update member roles
    Json(role): Json<Role>,
) -> ApiResult<()> {
    user.has_org_admin_access(&org_id)?;

    if *user.id() == user_id {
        prevent_last_remaining_admin(&repo, &org_id, &user_id).await?;
    }

    repo.update_member_role(org_id, user_id, role).await?;

    info!(
        updated_user_id = user_id.to_string(),
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "updated organization member",
    );

    Ok(Json(()))
}

pub async fn update_block_status(
    Path(org_id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser, // only users (super admins) are allowed to update block status
    Json(block_status): Json<OrgBlockStatus>,
) -> ApiResult<Organization> {
    user.is_super_admin()
        .then_some(())
        .ok_or(ApiError::Forbidden)?;

    let organization = repo.update_block_status(org_id, block_status).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        block_status = block_status.to_string(),
        "updated organization block status",
    );

    Ok(Json(organization))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::{
        api::{
            tests::{TestServer, deserialize_body, serialize_body},
            whoami::WhoamiResponse,
        },
        models::{OrgRole, Role},
    };

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_list_organizations(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let org_2 = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap();
        let mut server = TestServer::new(pool.clone(), Some(user_1)).await;

        // users should be able to list their organizations
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 2);
        assert!(organizations.iter().any(|o| o.id() == org_1));
        assert!(organizations.iter().any(|o| o.id() == org_2));

        // users without organizations don't see any organizations
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // has no organizations
        server.set_user(Some(user_3));
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 0);

        // super admin should be able to list all organizations
        let super_admin = "deadbeef-4e43-4a66-bbb9-fbcd4a933a34".parse().unwrap();
        server.set_user(Some(super_admin));
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 8);

        // not logged in users can't list organizations
        server.set_user(None);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_lifecycle(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // has no organizations
        let server = TestServer::new(pool.clone(), Some(user_3)).await;

        // start with no organizations
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 0);

        // create an organization
        let response = server
            .post(
                "/api/organizations",
                serialize_body(&NewOrganization {
                    name: "Test Org".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_org: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(created_org.name, "Test Org");

        // get organization
        let response = server
            .get(format!("/api/organizations/{}", created_org.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id(), created_org.id());
        assert_eq!(organization.name, "Test Org");

        // whoami should contain read-only role for created organization (until the user subscribed using moneybird)
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_3);
        assert_eq!(whoami.org_roles.len(), 1);
        assert_eq!(
            whoami.org_roles[0],
            OrgRole {
                role: Role::ReadOnly,
                org_id: created_org.id()
            }
        );

        // organization should contain the user as read-only
        let response = server
            .get(format!("/api/organizations/{}/members", created_org.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let members: Vec<OrganizationMember> = deserialize_body(response.into_body()).await;
        assert_eq!(members.len(), 1);
        assert_eq!(*members[0].user_id(), user_3);

        // Get link to choose subscription
        let response = server
            .get(format!(
                "/api/organizations/{}/subscription/new",
                created_org.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Reload subscriptions (in non-production environment, this activates a mock subscription)
        let response = server
            .get(format!(
                "/api/organizations/{}/subscription",
                created_org.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // whoami should contain admin role for created organization now
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id, user_3);
        assert_eq!(whoami.org_roles.len(), 1);
        assert_eq!(
            whoami.org_roles[0],
            OrgRole {
                role: Role::Admin,
                org_id: created_org.id()
            }
        );

        // update organization
        let response = server
            .put(
                format!("/api/organizations/{}", created_org.id()),
                serialize_body(&NewOrganization {
                    name: "Updated Org".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let updated_org: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(updated_org.id(), created_org.id());
        assert_eq!(updated_org.name, "Updated Org");

        // get organization
        let response = server
            .get(format!("/api/organizations/{}", created_org.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organization: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(organization.id(), created_org.id());
        assert_eq!(organization.name, "Updated Org");

        // remove organization
        let response = server
            .delete(format!("/api/organizations/{}", created_org.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify organization is removed
        let response = server
            .get(format!("/api/organizations/{}", created_org.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 0);
    }

    async fn test_organization_no_access(
        server: &TestServer,
        read_status_code: StatusCode,
        write_status_code: StatusCode,
    ) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235"; // is admin of org 1 and 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";

        // can't get organization
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't update organization
        let response = server
            .put(
                format!("/api/organizations/{org_1}"),
                serialize_body(&NewOrganization {
                    name: "Updated Org".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't remove organization
        let response = server
            .delete(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't get organization members
        let response = server
            .get(format!("/api/organizations/{org_1}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't remove organization member
        let response = server
            .delete(format!("/api/organizations/{org_1}/members/{user_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't update organization member's role
        let response = server
            .put(
                format!("/api/organizations/{org_1}/members/{user_1}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // nobody (except super admins) can update the organization's block status
        let response = server
            .put(
                format!("/api/organizations/{org_1}/admin"),
                serialize_body(OrgBlockStatus::NoSendingOrReceiving),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access_wrong_user(pool: PgPool) {
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let server = TestServer::new(pool.clone(), Some(user_2)).await;
        test_organization_no_access(&server, StatusCode::FORBIDDEN, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access_read_only(pool: PgPool) {
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e".parse().unwrap(); // is read-only in org 1
        let server = TestServer::new(pool.clone(), Some(user_5)).await;
        test_organization_no_access(&server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access_maintainer(pool: PgPool) {
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let server = TestServer::new(pool.clone(), Some(user_4)).await;
        test_organization_no_access(&server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access_not_logged_in(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;
        test_organization_no_access(&server, StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED)
            .await;

        // not logged in users can't create organization
        let response = server
            .post(
                "/api/organizations",
                serialize_body(&NewOrganization {
                    name: "Test Org".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access_admin(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let server = TestServer::new(pool.clone(), Some(user_1)).await;

        // nobody (except super admins) can update the organization's block status
        let response = server
            .put(
                format!("/api/organizations/{org_1}/admin"),
                serialize_body(OrgBlockStatus::NoSendingOrReceiving),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_members(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e"; // is read-only in org 1
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let org_2 = "5d55aec5-136a-407c-952f-5348d4398204";
        let mut server = TestServer::new(pool.clone(), Some(user_1)).await;

        // remove user 5 from org 1
        let response = server
            .delete(format!("/api/organizations/{org_1}/members/{user_5}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // org 1 now has 2 remaining members: an admin and a maintainer
        let response = server
            .get(format!("/api/organizations/{org_1}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let members: Vec<OrganizationMember> = deserialize_body(response.into_body()).await;
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|u| *u.user_id() == user_1));

        // can't remove themselves from org 1 as they are the last remaining admin
        let response = server
            .delete(format!("/api/organizations/{org_1}/members/{user_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);

        // can't make themselves non-admin as they are the last remaining admin
        let response = server
            .put(
                format!("/api/organizations/{org_1}/members/{user_1}"),
                serialize_body(Role::Maintainer),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);

        // org 2 has 2 admins
        let response = server
            .get(format!("/api/organizations/{org_2}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let members: Vec<OrganizationMember> = deserialize_body(response.into_body()).await;
        assert_eq!(members.len(), 2);

        // can remove themselves from org 2
        let response = server
            .delete(format!("/api/organizations/{org_2}/members/{user_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // now they lost access to org 2
        let response = server
            .get(format!("/api/organizations/{org_2}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // org 2 now contains only 1 member
        server.set_user(Some(user_2));
        let response = server
            .get(format!("/api/organizations/{org_2}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let members: Vec<OrganizationMember> = deserialize_body(response.into_body()).await;
        assert_eq!(members.len(), 1);
        assert_eq!(*members[0].user_id(), user_2);

        // put user_1 back in org 2
        let repo = OrganizationRepository::new(pool);
        repo.add_member(org_2.parse().unwrap(), user_1, Role::Admin)
            .await
            .unwrap();

        // user_1 can make user_2 non-admin in org 2
        server.set_user(Some(user_1));
        let response = server
            .put(
                format!("/api/organizations/{org_2}/members/{user_2}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify user_2 is read-only now
        let response = server
            .get(format!("/api/organizations/{org_2}/members"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let members: Vec<OrganizationMember> = deserialize_body(response.into_body()).await;
        assert!(
            members
                .iter()
                .filter(|m| *m.user_id() == user_2)
                .all(|m| *m.role() == Role::ReadOnly)
        );

        // make user_2 admin again
        let response = server
            .put(
                format!("/api/organizations/{org_2}/members/{user_2}"),
                serialize_body(Role::Admin),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // user_1 can make themselves non-admin in org 2
        let response = server
            .put(
                format!("/api/organizations/{org_2}/members/{user_1}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // can't remove users anymore
        let response = server
            .delete(format!("/api/organizations/{org_2}/members/{user_2}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
