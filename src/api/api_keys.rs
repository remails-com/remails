use super::error::{ApiError, ApiResult};
use crate::{
    api::auth::Authenticated,
    models::{ApiKey, ApiKeyId, ApiKeyRepository, ApiKeyRequest, ApiUser, OrganizationId},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

pub async fn create_api_key(
    State(repo): State<ApiKeyRepository>,
    user: ApiUser,
    Path(org_id): Path<OrganizationId>,
    Json(request): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
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

pub async fn update_api_key(
    State(repo): State<ApiKeyRepository>,
    user: ApiUser,
    Path((org_id, api_key_id)): Path<(OrganizationId, ApiKeyId)>,
    Json(request): Json<ApiKeyRequest>,
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

pub async fn list_api_keys(
    State(repo): State<ApiKeyRepository>,
    Path(org_id): Path<OrganizationId>,
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
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::{CreatedApiKeyWithPassword, Role},
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
            description: "Test Credential".to_string(),
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
                    role: Role::Maintainer,
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
}
