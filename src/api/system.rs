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

/// Get Remails current runtime configuration
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

/// Update Remails current runtime configuration
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
