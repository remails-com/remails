use crate::{api::oauth, models, models::Error};
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use thiserror::Error;
use tracing::error;

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Database(#[from] models::Error),
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("unauthorized")]
    Unauthorized,
    #[error("OAuth error: {0}")]
    OAuth(#[from] oauth::Error),
    #[error("{0}")]
    Serialization(#[from] serde_json::Error),
    #[error("{0}")]
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response<axum::body::Body> {
        error!("API server error: {self} {self:?}");

        let (status, message) = match self {
            ApiError::Database(db) => match db {
                Error::Serialization(err) => {
                    error!("{err}");
                    (StatusCode::BAD_REQUEST, err.to_string())
                }
                Error::NotFound(err) => {
                    error!("{err}");
                    (StatusCode::NOT_FOUND, "Not found".to_string())
                }
                Error::Conflict => (StatusCode::CONFLICT, "Conflict".to_string()),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                ),
            },
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::OAuth(err) => (err.status_code(), err.user_message()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Serialization(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::BadRequest(err) => (StatusCode::BAD_REQUEST, err),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
