use super::error::{AppError, ApiResult};
use crate::{
    api::{ApiState, auth::Authenticated, validation::ValidatedJson},
    models::{
        ApiKey, ApiKeyId, ApiKeyRepository, ApiKeyRequest, ApiUser, CreatedApiKeyWithPassword,
        OrganizationId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(create_api_key, list_api_keys))
        .routes(routes!(update_api_key, remove_api_key))
}

/// Create new API key
#[utoipa::path(post, path = "/organizations/{org_id}/api_keys",
    tags = ["internal", "Api Key"],
    request_body = ApiKeyRequest,
    responses(
        (status = 201, description = "Successfully created API key", body = CreatedApiKeyWithPassword),
        AppError,
    )
)]
pub async fn create_api_key(
    State(repo): State<ApiKeyRepository>,
    user: ApiUser,
    Path((org_id,)): Path<(OrganizationId,)>,
    ValidatedJson(request): ValidatedJson<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    user.has_org_write_access(&org_id)?;

    let new_api_key = repo.create(org_id, &request).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        api_key_id = new_api_key.id().to_string(),
        description = new_api_key.description(),
        "created API key"
    );

    Ok((StatusCode::CREATED, Json(new_api_key)))
}

/// Update API key
#[utoipa::path(put, path = "/organizations/{org_id}/api_keys/{api_key_id}",
    tags = ["internal", "Api Key"],
    request_body = ApiKeyRequest,
    responses(
        (status = 200, description = "Successfully updated API key", body = ApiKey),
        AppError,
    )
)]
pub async fn update_api_key(
    State(repo): State<ApiKeyRepository>,
    user: ApiUser,
    Path((org_id, api_key_id)): Path<(OrganizationId, ApiKeyId)>,
    ValidatedJson(request): ValidatedJson<ApiKeyRequest>,
) -> ApiResult<ApiKey> {
    user.has_org_write_access(&org_id)?;

    let update = repo.update(org_id, api_key_id, &request).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        api_key_id = update.id().to_string(),
        api_key_description = update.description(),
        "updated API key"
    );

    Ok(Json(update))
}

/// List existing API keys
#[utoipa::path(get, path = "/organizations/{org_id}/api_keys",
    tags = ["internal", "Api Key"],
    responses(
        (status = 200, description = "Successfully fetched API keys", body = Vec<ApiKey>),
        AppError,
    )
)]
pub async fn list_api_keys(
    State(repo): State<ApiKeyRepository>,
    Path((org_id,)): Path<(OrganizationId,)>,
    user: ApiUser,
) -> ApiResult<Vec<ApiKey>> {
    user.has_org_read_access(&org_id)?;

    let api_keys = repo.list(org_id).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "listed {} API keys",
        api_keys.len()
    );

    Ok(Json(api_keys))
}

/// Delete an API key
#[utoipa::path(delete, path = "/organizations/{org_id}/api_keys/{api_key_id}",
    tags = ["internal", "Api Key"],
    responses(
        (status = 200, description = "Successfully deleted API key", body = ApiKeyId),
        AppError,
    )
)]
pub async fn remove_api_key(
    State(repo): State<ApiKeyRepository>,
    user: ApiUser,
    Path((org_id, api_key_id)): Path<(OrganizationId, ApiKeyId)>,
) -> ApiResult<ApiKeyId> {
    user.has_org_write_access(&org_id)?;

    let api_key_id = repo.remove(org_id, api_key_id).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        credential_id = api_key_id.to_string(),
        "deleted API key",
    );

    Ok(Json(api_key_id))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use base64ct::Encoding;
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::{
            ApiDomain, ApiMessage, ApiMessageMetadata, CreatedApiKeyWithPassword, NewOrganization,
            Organization, Project, Role, Stream,
        },
    };

    use super::*;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams",)
    ))]
    async fn test_api_key_lifecycle(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let server = TestServer::new(pool.clone(), Some(user_1)).await;

        // start with no API keys
        let response = server
            .get(format!("/api/organizations/{org_1}/api_keys"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let api_keys: Vec<ApiKey> = deserialize_body(response.into_body()).await;
        assert_eq!(api_keys.len(), 0);

        // create an API key
        let new_key = ApiKeyRequest {
            description: "Test API Key".to_string(),
            role: Role::Maintainer,
        };
        let response = server
            .post(
                format!("/api/organizations/{org_1}/api_keys"),
                serialize_body(&new_key),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_key: CreatedApiKeyWithPassword = deserialize_body(response.into_body()).await;
        assert_eq!(created_key.description(), new_key.description);
        assert_eq!(*created_key.role(), new_key.role);
        assert_eq!(created_key.organization_id().to_string(), org_1);

        // list API keys
        let response = server
            .get(format!("/api/organizations/{org_1}/api_keys"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let api_keys: Vec<ApiKey> = deserialize_body(response.into_body()).await;
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id(), created_key.id());
        assert_eq!(api_keys[0].description(), created_key.description());
        assert_eq!(api_keys[0].role(), created_key.role());

        // update API key
        let updated_key = ApiKeyRequest {
            description: "Updated Key".to_string(),
            role: Role::ReadOnly,
        };
        let response = server
            .put(
                format!("/api/organizations/{org_1}/api_keys/{}", created_key.id()),
                serialize_body(&updated_key),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let api_key: ApiKey = deserialize_body(response.into_body()).await;
        assert_eq!(api_key.description(), updated_key.description);
        assert_eq!(*api_key.role(), updated_key.role);
        assert_eq!(api_key.id(), created_key.id());

        // list API keys
        let response = server
            .get(format!("/api/organizations/{org_1}/api_keys"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let api_keys: Vec<ApiKey> = deserialize_body(response.into_body()).await;
        assert_eq!(api_keys.len(), 1);
        assert_eq!(api_keys[0].id(), created_key.id());
        assert_eq!(api_keys[0].description(), updated_key.description);
        assert_eq!(*api_keys[0].role(), updated_key.role);

        // remove API key
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/api_keys/{}",
                created_key.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let deleted_id: ApiKeyId = deserialize_body(response.into_body()).await;
        assert_eq!(deleted_id, *created_key.id());

        // check if API key is deleted
        let response = server
            .get(format!("/api/organizations/{org_1}/api_keys"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let api_keys: Vec<ApiKey> = deserialize_body(response.into_body()).await;
        assert_eq!(api_keys.len(), 0);
    }

    async fn test_api_key_no_access(
        server: TestServer,
        read_status_code: StatusCode,
        write_status_code: StatusCode,
    ) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let api_key_id = "951ec618-bcc9-4224-9cf1-ed41a84f41d8"; // maintainer API key in org 1

        // can't list API keys
        let response = server
            .get(format!("/api/organizations/{org_1}/api_keys"))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't create API keys
        let response = server
            .post(
                format!("/api/organizations/{org_1}/api_keys"),
                serialize_body(&ApiKeyRequest {
                    description: "Test Key".to_string(),
                    role: Role::ReadOnly,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't update API keys
        let response = server
            .put(
                format!("/api/organizations/{org_1}/api_keys/{api_key_id}"),
                serialize_body(&ApiKeyRequest {
                    description: "Updated Credential".to_string(),
                    role: Role::Maintainer,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't delete API keys
        let response = server
            .delete(format!("/api/organizations/{org_1}/api_keys/{api_key_id}"))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams", "api_keys")
    ))]
    async fn test_api_key_no_access_wrong_user(pool: PgPool) {
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let server = TestServer::new(pool.clone(), Some(user_2)).await;
        test_api_key_no_access(server, StatusCode::FORBIDDEN, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams", "api_keys")
    ))]
    async fn test_api_key_no_access_read_only(pool: PgPool) {
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e".parse().unwrap(); // is read only in org 1
        let server = TestServer::new(pool.clone(), Some(user_5)).await;
        test_api_key_no_access(server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams", "api_keys")
    ))]
    async fn test_api_key_no_access_not_logged_in(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;
        test_api_key_no_access(server, StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED).await;
    }

    impl TestServer {
        pub async fn use_api_key(&mut self, org_id: OrganizationId, role: Role) -> ApiKeyId {
            // request an API key using the currently logged-in user
            let response = self
                .post(
                    format!("/api/organizations/{org_id}/api_keys"),
                    serialize_body(&ApiKeyRequest {
                        description: "Test API Key".to_string(),
                        role,
                    }),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
            let created_key: CreatedApiKeyWithPassword =
                deserialize_body(response.into_body()).await;

            // log out user and start using API key
            self.set_user(None);
            let base64_credentials = base64ct::Base64::encode_string(
                format!("{}:{}", created_key.id(), created_key.password()).as_bytes(),
            );
            self.headers
                .insert("Authorization", format!("Basic {base64_credentials}"));

            *created_key.id()
        }
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages",
            "org_domains",
            "proj_domains"
        )
    ))]
    async fn test_maintainer_api_key_use_api(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), Some(user_4)).await;
        server.use_api_key(org_1, Role::Maintainer).await;

        // list organizations
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let orgs: Vec<Organization> = deserialize_body(response.into_body()).await;
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].id(), org_1);

        // get specific organization
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let org: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(org.id(), org_1);

        // list organization domains
        let org_dom_1 = "ed28baa5-57f7-413f-8c77-7797ba6a8780".parse().unwrap();
        let response = server
            .get(format!("/api/organizations/{org_1}/domains"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domains: Vec<ApiDomain> = deserialize_body(response.into_body()).await;
        assert_eq!(domains.len(), 2);
        assert!(domains.iter().any(|d| d.id() == org_dom_1));

        // list projects
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let proj_2 = "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap();
        let response = server
            .get(format!("/api/organizations/{org_1}/projects"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let projects: Vec<Project> = deserialize_body(response.into_body()).await;
        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.id() == proj_1));
        assert!(projects.iter().any(|p| p.id() == proj_2));

        // list streams
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let streams: Vec<Stream> = deserialize_body(response.into_body()).await;
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].id(), stream_1);

        // list messages
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let messages: Vec<ApiMessageMetadata> = deserialize_body(response.into_body()).await;
        assert_eq!(messages.len(), 5);

        // get a specific message
        let message_1 = "e165562a-fb6d-423b-b318-fd26f4610634".parse().unwrap();
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let message: ApiMessage = deserialize_body(response.into_body()).await;
        assert_eq!(message.id(), message_1);

        // delete a message
        let response = server.delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // check that is was removed
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "api_keys",
            "projects",
            "streams",
            "smtp_credentials",
            "messages",
            "invites"
        )
    ))]
    async fn test_api_keys_should_not(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), Some(user_4)).await;
        server.use_api_key(org_1, Role::Maintainer).await;

        // API keys should NOT be able to create API keys
        let response = server
            .post(
                format!("/api/organizations/{org_1}/api_keys"),
                serialize_body(&ApiKeyRequest {
                    description: "Test API Key".to_string(),
                    role: Role::ReadOnly,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to delete API keys
        let api_key_1 = "951ec618-bcc9-4224-9cf1-ed41a84f41d8";
        let response = server
            .delete(format!("/api/organizations/{org_1}/api_keys/{api_key_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to update API keys
        let response = server
            .put(
                format!("/api/organizations/{org_1}/api_keys/{api_key_1}"),
                serialize_body(&ApiKeyRequest {
                    description: "Updated API Key".to_string(),
                    role: Role::ReadOnly,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to create invites
        let response = server
            .post(
                format!("/api/invite/{org_1}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to delete invites
        let invite_1 = "32bba198-fdd8-4cb7-8b82-85857dd2527f";
        let response = server
            .delete(format!("/api/invite/{org_1}/{invite_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to accept invites
        let response = server
            .post(
                format!("/api/invite/{org_1}/{invite_1}/unsecure"),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to remove members
        let response = server
            .delete(format!("/api/organizations/{org_1}/members/{user_4}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to update members
        let response = server
            .put(
                format!("/api/organizations/{org_1}/members/{user_4}"),
                serialize_body(Role::ReadOnly),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // API keys should NOT be able to create organizations
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

        // API keys should NOT be able to remove organizations
        let response = server
            .delete(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages",
            "org_domains",
            "proj_domains"
        )
    ))]
    async fn test_read_only_api_keys(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // is maintainer of org 1
        let mut server = TestServer::new(pool.clone(), Some(user_4)).await;
        server.use_api_key(org_1, Role::ReadOnly).await; // read-only API key

        // Read-only API keys should not be able to delete projects
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462";
        let response = server
            .delete(format!("/api/organizations/{org_1}/projects/{proj_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Read-only API keys should not be able to delete streams
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a";
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Read-only API keys should not be able to delete messages
        let message_1 = "e165562a-fb6d-423b-b318-fd26f4610634";
        let response = server.delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Read-only API keys are able to get organization
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Read-only API keys are able to list messages
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Read-only API keys are able to view messages
        let response = server.get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/messages/{message_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Read-only API keys are able to view organization domains
        let org_dom_1 = "ed28baa5-57f7-413f-8c77-7797ba6a8780";
        let response = server
            .get(format!("/api/organizations/{org_1}/domains/{org_dom_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Read-only API keys are able to view project domains
        let proj_dom_1 = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a";
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/domains/{proj_dom_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_keys",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials",
            "messages",
        )
    ))]
    async fn test_invalid_api_keys(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let mut server = TestServer::new(pool.clone(), None).await;

        // not basic auth
        server
            .headers
            .insert("Authorization", "Bearer aGFoYSB5ZXM=".to_owned());
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // nonsense credentials
        server
            .headers
            .insert("Authorization", "Basic aGFoYSB5ZXM=".to_owned());
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // invalid API key id
        let base64_credentials = base64ct::Base64::encode_string(
            "000000000-0000-0000-0000-000000000000:unsecure".as_bytes(),
        );
        server
            .headers
            .insert("Authorization", format!("Basic {base64_credentials}"));
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // incorrect password
        let base64_credentials = base64ct::Base64::encode_string(
            "951ec618-bcc9-4224-9cf1-ed41a84f41d8:incorrect".as_bytes(),
        );
        server
            .headers
            .insert("Authorization", format!("Basic {base64_credentials}"));
        let response = server
            .get(format!("/api/organizations/{org_1}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
