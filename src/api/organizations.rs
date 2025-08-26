use crate::{
    api::error::{ApiError, ApiResult},
    models::{ApiUser, NewOrganization, Organization, OrganizationId, OrganizationRepository},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

fn has_read_access(user: &ApiUser, org: &OrganizationId) -> Result<(), ApiError> {
    has_write_access(user, org)
}

fn has_write_access(user: &ApiUser, org: &OrganizationId) -> Result<(), ApiError> {
    if user.is_org_admin(org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn list_organizations(
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
) -> ApiResult<Vec<Organization>> {
    let filter = if user.is_super_admin() {
        None // show all organizations
    } else {
        Some(user.viewable_organizations())
    };
    let organizations = repo.list(filter).await?;

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
    has_read_access(&user, &id)?;

    let organization = repo.get_by_id(id).await?.ok_or(ApiError::NotFound)?;

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
    has_write_access(&user, &id)?;

    let organization_id = repo.remove(id).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = organization_id.to_string(),
        "deleted organization",
    );

    Ok(Json(organization_id))
}

pub async fn update_organization(
    Path(id): Path<OrganizationId>,
    State(repo): State<OrganizationRepository>,
    user: ApiUser,
    Json(update): Json<NewOrganization>,
) -> ApiResult<Organization> {
    has_write_access(&user, &id)?;

    let organization = repo.update(id, update).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = organization.id().to_string(),
        "updated organization",
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
        let mut server = TestServer::new(pool.clone(), user_1).await;

        // users should be able to list their organizations
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 2);
        assert!(organizations.iter().any(|o| o.id() == org_1));
        assert!(organizations.iter().any(|o| o.id() == org_2));

        // users without organizations don't see any organizations
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // has no organizations
        server.set_user(user_3);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 0);

        // super admin should be able to list all organizations
        let super_admin = "deadbeef-4e43-4a66-bbb9-fbcd4a933a34".parse().unwrap();
        server.set_user(super_admin);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let organizations: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(organizations.len(), 6);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_lifecycle(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // has no organizations
        let server = TestServer::new(pool.clone(), user_3).await;

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

        // whoami should contain admin role for created organization
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
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

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_organization_no_access(pool: PgPool) {
        let user_b = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let server = TestServer::new(pool.clone(), user_b).await;

        // can't get organization
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

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
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't remove organization
        let response = server
            .delete(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
