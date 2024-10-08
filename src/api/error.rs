use axum::{http::StatusCode, response::IntoResponse, Json};
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response<axum::body::Body> {
        error!("API server error: {self} {self:?}");

        let (status, message) = match self {
            ApiError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
