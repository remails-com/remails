use crate::api::oauth;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use thiserror::Error;
use tracing::error;

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("OAuth error: {0}")]
    OAuth(#[from] oauth::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response<axum::body::Body> {
        error!("API server error: {self} {self:?}");

        let (status, message) = match self {
            ApiError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::OAuth(err) => (err.status_code(), err.user_message()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
