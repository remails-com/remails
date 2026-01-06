use crate::{api::oauth, models, models::Error};
use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use std::collections::BTreeMap;
use tokio::task::JoinError;
use tracing::{error, info, warn};
use utoipa::{
    IntoResponses,
    openapi::{RefOr, Response},
};
use uuid::Uuid;

pub type ApiResult<T> = Result<Json<T>, AppError>;

#[derive(thiserror::Error, Debug, derive_more::Display)]
pub enum AppError {
    BadRequest(String),
    NotFound,
    Conflict(String),
    TooManyRequests,
    Internal,
    Forbidden,
    Unauthorized,
    BadGateway,
    PayloadTooLarge,
    RequestTimeout,
}

#[derive(utoipa::IntoResponses, Serialize)]
enum ApiError {
    /// Bad Request
    #[response(status = BAD_REQUEST)]
    BadRequest(ApiErrorResponse),
    /// Not Found
    #[response(status = NOT_FOUND)]
    NotFound(ApiErrorResponse),
    /// Conflict
    #[response(status = CONFLICT)]
    Conflict(ApiErrorResponse),
    /// Too Many Requests
    #[response(status = TOO_MANY_REQUESTS)]
    TooManyRequests(ApiErrorResponse),
    /// Internal Server Error
    #[response(status = INTERNAL_SERVER_ERROR)]
    Internal(ApiErrorResponse),
    /// Forbidden
    #[response(status = FORBIDDEN)]
    Forbidden(ApiErrorResponse),
    /// Unauthorized
    #[response(status = UNAUTHORIZED)]
    Unauthorized(ApiErrorResponse),
    /// Bad Gateway
    #[response(status = BAD_GATEWAY)]
    BadGateway(ApiErrorResponse),
    /// Payload Too Large
    #[response(status = PAYLOAD_TOO_LARGE)]
    PayloadTooLarge(ApiErrorResponse),
    /// Request Timeout
    #[response(status = REQUEST_TIMEOUT)]
    RequestTimeout(ApiErrorResponse),
}

#[derive(Serialize, utoipa::ToResponse, utoipa::ToSchema)]
#[response(description = "API error details")]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct ApiErrorResponse {
    description: String,
    reference: Uuid,
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        let reference = Uuid::new_v4();
        info!(
            error_reference = reference.to_string(),
            "API server error: {err:?}"
        );

        let content = ApiErrorResponse {
            description: err.to_string(),
            reference,
        };

        match err {
            AppError::BadRequest(_) => ApiError::BadRequest(content),
            AppError::NotFound => ApiError::NotFound(content),
            AppError::Conflict(_) => ApiError::Conflict(content),
            AppError::TooManyRequests => ApiError::TooManyRequests(content),
            AppError::Internal => ApiError::Internal(content),
            AppError::Forbidden => ApiError::Forbidden(content),
            AppError::Unauthorized => ApiError::Unauthorized(content),
            AppError::BadGateway => ApiError::BadGateway(content),
            AppError::PayloadTooLarge => ApiError::PayloadTooLarge(content),
            AppError::RequestTimeout => ApiError::RequestTimeout(content),
        }
    }
}

impl From<oauth::Error> for AppError {
    fn from(err: oauth::Error) -> Self {
        let message = err.user_message();
        error!("API server error (OAuth): {message} {err:?}");

        match err {
            oauth::Error::MissingEnvironmentVariable(_)
            | oauth::Error::Json(_)
            | oauth::Error::Database(_)
            | oauth::Error::Other(_) => AppError::Internal,
            oauth::Error::FetchUser(_) | oauth::Error::ParseUser(_) => AppError::BadGateway,
            oauth::Error::OauthToken(_)
            | oauth::Error::MissingCSRFCookie
            | oauth::Error::CSRFTokenMismatch => AppError::Unauthorized,
            oauth::Error::Conflict(_) => AppError::Conflict(message),
            oauth::Error::Forbidden => AppError::Forbidden,
        }
    }
}

impl From<JoinError> for AppError {
    fn from(err: JoinError) -> Self {
        error!("{err:?}");
        AppError::Internal
    }
}

impl From<crate::moneybird::Error> for AppError {
    fn from(err: crate::moneybird::Error) -> Self {
        error!("Moneybird error: {err} {err:?}");

        AppError::BadGateway
    }
}

impl From<garde::Report> for AppError {
    fn from(err: garde::Report) -> Self {
        warn!("validation error: {err} {err:?}");

        AppError::BadRequest(err.to_string())
    }
}

impl From<JsonRejection> for AppError {
    fn from(err: JsonRejection) -> Self {
        warn!("Json error: {err} {err:?}");

        AppError::BadRequest(err.to_string())
    }
}

impl From<QueryRejection> for AppError {
    fn from(err: QueryRejection) -> Self {
        warn!("API server error: {err} {err:?}");

        AppError::BadRequest(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        error!("API server error: {err} {err:?}");

        AppError::Internal
    }
}

impl From<models::Error> for AppError {
    fn from(err: models::Error) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        match err {
            Error::Serialization(_) => AppError::BadRequest(err.to_string()),
            Error::NotFound(_) => AppError::NotFound,
            Error::ForeignKeyViolation => AppError::BadRequest("Foreign key violation".to_string()),
            Error::Conflict => AppError::Conflict("Conflict".to_string()),
            Error::BadRequest(err) => AppError::BadRequest(err.to_string()),
            Error::TooManyRequests => AppError::TooManyRequests,
            Error::OrgBlocked => AppError::Forbidden,
            _ => AppError::Internal,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        Into::<ApiError>::into(self).into_response()
    }
}

impl IntoResponses for AppError {
    fn responses() -> BTreeMap<String, RefOr<Response>> {
        ApiError::responses()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::BadRequest(body) => (StatusCode::BAD_REQUEST, Json(body)),
            ApiError::NotFound(body) => (StatusCode::NOT_FOUND, Json(body)),
            ApiError::Conflict(body) => (StatusCode::CONFLICT, Json(body)),
            ApiError::TooManyRequests(body) => (StatusCode::TOO_MANY_REQUESTS, Json(body)),
            ApiError::Internal(body) => (StatusCode::INTERNAL_SERVER_ERROR, Json(body)),
            ApiError::Forbidden(body) => (StatusCode::FORBIDDEN, Json(body)),
            ApiError::Unauthorized(body) => (StatusCode::UNAUTHORIZED, Json(body)),
            ApiError::BadGateway(body) => (StatusCode::BAD_GATEWAY, Json(body)),
            ApiError::PayloadTooLarge(body) => (StatusCode::PAYLOAD_TOO_LARGE, Json(body)),
            ApiError::RequestTimeout(body) => (StatusCode::REQUEST_TIMEOUT, Json(body)),
        }
        .into_response()
    }
}
