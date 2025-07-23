use crate::{api::oauth, models, models::Error};
use axum::{Json, extract::rejection::JsonRejection, http::StatusCode, response::IntoResponse};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error};

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
    #[error("{0}")]
    PreconditionFailed(String),
    #[error("Moneybird: {0}")]
    Moneybird(#[from] crate::moneybird::Error),
}

impl From<garde::Report> for ApiError {
    fn from(err: garde::Report) -> Self {
        Self::BadRequest(err.to_string())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.to_string())
    }
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
                    debug!("{err}");
                    (StatusCode::NOT_FOUND, "Not found".to_string())
                }
                Error::ForeignKeyViolation => {
                    debug!("ForeignKeyViolation");
                    (StatusCode::BAD_REQUEST, "Foreign key violation".to_string())
                }
                Error::Conflict => (StatusCode::CONFLICT, "Conflict".to_string()),
                Error::BadRequest(err) => (StatusCode::BAD_REQUEST, err.to_string()),
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
            ApiError::PreconditionFailed(err) => (StatusCode::PRECONDITION_FAILED, err),
            ApiError::Moneybird(err) => match err {
                crate::moneybird::Error::Unauthorized => {
                    (StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                ),
            },
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
