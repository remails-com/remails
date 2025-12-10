use crate::{
    api::{
        ApiState, RemailsConfig,
        error::{ApiResult, AppError},
        validation::ValidatedJson,
    },
    models::{ApiUser, RuntimeConfig, RuntimeConfigRepository, RuntimeConfigResponse},
};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use tracing::{error, info, trace, warn};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(config))
        .routes(routes!(healthy))
        .routes(routes!(runtime_config))
        .routes(routes!(update_runtime_config))
}

/// Get runtime configuration
#[utoipa::path(get, path = "/config/runtime",
    tags = ["internal", "Misc"],
    security(("cookieAuth" = [])),
    responses(
        (status = 200, description = "Successfully fetched runtime config", body = RuntimeConfigResponse),
        AppError
    )
)]
async fn runtime_config(
    State(repo): State<RuntimeConfigRepository>,
    user: ApiUser,
) -> ApiResult<RuntimeConfigResponse> {
    if !user.is_super_admin() {
        warn!(
            user_id = user.id().to_string(),
            "User is not permitted to fetch the runtime configuration"
        );
        return Err(AppError::Forbidden);
    }

    let config = repo.get().await?;
    trace!(user_id = user.id().to_string(), "Fetched runtime config");

    Ok(Json(config))
}

/// Update runtime configuration
#[utoipa::path(put, path = "/config/runtime",
    tags = ["internal", "Misc"],
    security(("cookieAuth" = [])),
    request_body = RuntimeConfig,
    responses(
        (status = 200, description = "Successfully updated runtime config", body = RuntimeConfigResponse),
        AppError
    )
)]
async fn update_runtime_config(
    State(repo): State<RuntimeConfigRepository>,
    user: ApiUser,
    ValidatedJson(config): ValidatedJson<RuntimeConfig>,
) -> ApiResult<RuntimeConfigResponse> {
    if !user.is_super_admin() {
        warn!(
            user_id = user.id().to_string(),
            "User is not permitted to update the runtime configuration"
        );
        return Err(AppError::Forbidden);
    }

    let config = repo.update(config).await?;
    info!(user_id = user.id().to_string(), "Updated runtime config");

    Ok(Json(config))
}

#[derive(Debug, Serialize, ToSchema)]
struct HealthyResponse {
    healthy: bool,
    status: &'static str,
}

/// Remails health check
#[utoipa::path(get, path = "/healthy",
    tags = ["internal", "Misc"],
    security(()),
    responses(
        (status = 200, description = "Remails health status", body = HealthyResponse),
    )
)]
async fn healthy(State(pool): State<PgPool>) -> Json<HealthyResponse> {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => Json(HealthyResponse {
            healthy: true,
            status: "OK",
        }),
        Err(e) => {
            error!("database error: {:?}", e);

            Json(HealthyResponse {
                healthy: false,
                status: "database error",
            })
        }
    }
}

/// Remails configuration
///
/// Get the configuration and environment details of the Remails server
#[utoipa::path(get, path = "/config",
    security(()),
    tags = ["Misc"],
    responses(
        (status = 200, description = "Remails configuration", body = RemailsConfig),
    )
)]
pub async fn config(State(config): State<RemailsConfig>) -> Response {
    Json(config).into_response()
}

#[cfg(test)]
mod tests {
    use crate::{
        api::{
            RemailsConfig,
            tests::{TestServer, deserialize_body, serialize_body},
        },
        models::{RuntimeConfig, RuntimeConfigResponse},
    };
    use axum::body::Body;
    use http::StatusCode;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn test_util_endpoints(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        // can access health check
        let response = server.get("/api/healthy").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        assert!(bytes.iter().eq(br#"{"healthy":true,"status":"OK"}"#));

        // can access Remails config
        let response = server.get("/api/config").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _: RemailsConfig = deserialize_body(response.into_body()).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn runtime_config(pool: PgPool) {
        let invalid_project = "a6c2e1f0-60a8-4db0-9223-387d5d0eecc0".parse().unwrap();
        let project1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let org1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();

        let server = TestServer::new(
            pool.clone(),
            // Use super admin for authorization
            Some("deadbeef-4e43-4a66-bbb9-fbcd4a933a34".parse().unwrap()),
        )
        .await;

        // Initially, the response should be empty
        let response = server.get("/api/config/runtime").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let config: RuntimeConfigResponse = deserialize_body(response.into_body()).await;
        assert_eq!(config, RuntimeConfigResponse::new(None, None, None, None));

        // Update the runtime with a non-existent project
        let response = server
            .put(
                "/api/config/runtime",
                serialize_body(RuntimeConfig::new(
                    Some(invalid_project),
                    Some("some@email.com".to_string()),
                )),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Update the with bad email
        let response = server
            .put(
                "/api/config/runtime",
                serialize_body(RuntimeConfig::new(
                    Some(project1),
                    Some("someemail.com".to_string()),
                )),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Update with valid data
        let expected = RuntimeConfigResponse::new(
            Some(project1),
            Some("Project 1 Organization 1".to_string()),
            Some(org1),
            Some("some@email.com".to_string()),
        );

        let response = server
            .put(
                "/api/config/runtime",
                serialize_body(RuntimeConfig::new(
                    Some(project1),
                    Some("some@email.com".to_string()),
                )),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let config: RuntimeConfigResponse = deserialize_body(response.into_body()).await;
        assert_eq!(config, expected);

        // Validate update got actually stored
        let response = server.get("/api/config/runtime").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let config: RuntimeConfigResponse = deserialize_body(response.into_body()).await;
        assert_eq!(config, expected);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn runtime_config_only_for_super_admin(pool: PgPool) {
        // Start with no auth
        let mut server = TestServer::new(pool.clone(), None).await;
        let res = server.get("/api/config/runtime").await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let res = server
            .put("/api/config/runtime", Body::empty())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // user 1: admin of org 1 and org 2
        server.set_user(Some(
            "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(),
        ));
        let res = server.get("/api/config/runtime").await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let res = server
            .put(
                "/api/config/runtime",
                serialize_body(RuntimeConfig::new(
                    Some("a6c2e1f0-60a8-4db0-9223-387d5d0eecc0".parse().unwrap()),
                    Some("some@email.com".to_string()),
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }
}
