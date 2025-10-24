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
    #[error("too many requests, try again later")]
    TooManyRequests,
    #[error("OAuth error: {0}")]
    OAuth(#[from] oauth::Error),
    #[error("{0}")]
    Serialization(#[from] serde_json::Error),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    PreconditionFailed(&'static str),
    #[error("Moneybird: {0}")]
    Moneybird(#[from] crate::moneybird::Error),
    #[error("MessageBus: {0}")]
    MessageBus(reqwest::Error),
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

        let (status, message): (StatusCode, &str) = match self {
            ApiError::Database(db) => match db {
                Error::Serialization(err) => {
                    error!("{err}");
                    (StatusCode::BAD_REQUEST, &err.to_string())
                }
                Error::NotFound(err) => {
                    debug!("{err}");
                    (StatusCode::NOT_FOUND, "Not found")
                }
                Error::ForeignKeyViolation => {
                    debug!("ForeignKeyViolation");
                    (StatusCode::BAD_REQUEST, "Foreign key violation")
                }
                Error::Conflict => (StatusCode::CONFLICT, "Conflict"),
                Error::BadRequest(err) => (StatusCode::BAD_REQUEST, &err.to_string()),
                Error::TooManyRequests => {
                    debug!("Too many requests");
                    (StatusCode::TOO_MANY_REQUESTS, "Too many requests")
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            },
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            ApiError::OAuth(err) => (err.status_code(), &err.user_message()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::TooManyRequests => {
                debug!("Too many requests");
                (StatusCode::TOO_MANY_REQUESTS, "Too many requests")
            }
            ApiError::Serialization(err) => (StatusCode::BAD_REQUEST, &err.to_string()),
            ApiError::BadRequest(err) => (StatusCode::BAD_REQUEST, &err.clone()),
            ApiError::PreconditionFailed(err) => (StatusCode::PRECONDITION_FAILED, err),
            ApiError::Moneybird(err) => match err {
                crate::moneybird::Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            },
            ApiError::MessageBus(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error"),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
