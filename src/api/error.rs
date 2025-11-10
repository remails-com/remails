use crate::{api::oauth, models, models::Error};
use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use tokio::task::JoinError;
use tracing::{error, info, warn};
use uuid::Uuid;

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(utoipa::IntoResponses, Serialize)]
pub enum ApiError {
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
    InternalServerError(ApiErrorResponse),
    /// Forbidden
    #[response(status = FORBIDDEN)]
    Forbidden(ApiErrorResponse),
    /// Unauthorized
    #[response(status = UNAUTHORIZED)]
    Unauthorized(ApiErrorResponse),
    /// Precondition Failed
    #[response(status = PRECONDITION_FAILED)]
    PreconditionFailed(ApiErrorResponse),
    /// Bad Gateway
    #[response(status = BAD_GATEWAY)]
    BadGateway(ApiErrorResponse),
    /// Payload Too Large
    #[response(status = PAYLOAD_TOO_LARGE)]
    PayloadToLarge(ApiErrorResponse),
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

impl ApiError {
    pub fn unauthorized() -> Self {
        let reference = Uuid::new_v4();
        info!(
            error_reference = reference.to_string(),
            "API server error: Unauthorized"
        );

        ApiError::Unauthorized(ApiErrorResponse {
            description: "Unauthorized".to_string(),
            reference,
        })
    }

    pub fn request_timeout() -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: Request timeout"
        );

        ApiError::RequestTimeout(ApiErrorResponse {
            description: "Request timeout".to_string(),
            reference,
        })
    }

    pub fn payload_too_large() -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: Payload too large"
        );

        ApiError::PayloadToLarge(ApiErrorResponse {
            description: "Payload too large".to_string(),
            reference,
        })
    }

    pub fn bad_request(message: String) -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: Bad Request: {message}"
        );

        ApiError::BadRequest(ApiErrorResponse {
            description: message,
            reference,
        })
    }

    pub fn forbidden() -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: Forbidden"
        );

        ApiError::Forbidden(ApiErrorResponse {
            description: "Forbidden".to_string(),
            reference,
        })
    }

    pub fn precondition_failed(message: String) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: Precondition Failed: {message}"
        );

        ApiError::PreconditionFailed(ApiErrorResponse {
            description: message,
            reference,
        })
    }

    pub fn not_found() -> Self {
        let reference = Uuid::new_v4();
        info!(
            error_reference = reference.to_string(),
            "API server error: Not Found"
        );

        ApiError::NotFound(ApiErrorResponse {
            description: "Not found".to_string(),
            reference,
        })
    }

    pub fn too_many_requests() -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: Too many requests"
        );

        ApiError::TooManyRequests(ApiErrorResponse {
            description: "Too many requests".to_string(),
            reference,
        })
    }

    pub fn internal() -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: internal server error"
        );

        ApiError::InternalServerError(ApiErrorResponse {
            description: "Internal server error".to_string(),
            reference,
        })
    }
}

impl From<JoinError> for ApiError {
    fn from(err: JoinError) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "tokio task error: {err}"
        );

        ApiError::InternalServerError(ApiErrorResponse {
            description: "Internal server error".to_string(),
            reference,
        })
    }
}

impl From<oauth::Error> for ApiError {
    fn from(err: oauth::Error) -> Self {
        let message = err.user_message();
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error (OAuth): {message} {err:?}"
        );

        match err {
            oauth::Error::MissingEnvironmentVariable(_)
            | oauth::Error::Json(_)
            | oauth::Error::Database(_)
            | oauth::Error::Other(_) => ApiError::InternalServerError(ApiErrorResponse {
                description: message,
                reference,
            }),
            oauth::Error::FetchUser(_) | oauth::Error::ParseUser(_) => {
                ApiError::BadGateway(ApiErrorResponse {
                    description: message,
                    reference,
                })
            }
            oauth::Error::OauthToken(_)
            | oauth::Error::MissingCSRFCookie
            | oauth::Error::CSRFTokenMismatch => ApiError::Unauthorized(ApiErrorResponse {
                description: message,
                reference,
            }),
            oauth::Error::PreconditionFailed(_) => ApiError::PreconditionFailed(ApiErrorResponse {
                description: message,
                reference,
            }),
        }
    }
}

impl From<crate::moneybird::Error> for ApiError {
    fn from(err: crate::moneybird::Error) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        ApiError::BadGateway(ApiErrorResponse {
            description: err.to_string(),
            reference,
        })
    }
}

impl From<garde::Report> for ApiError {
    fn from(err: garde::Report) -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        ApiError::BadRequest(ApiErrorResponse {
            description: err.to_string(),
            reference,
        })
    }
}

impl From<JsonRejection> for ApiError {
    fn from(err: JsonRejection) -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        ApiError::BadRequest(ApiErrorResponse {
            description: err.to_string(),
            reference,
        })
    }
}

impl From<QueryRejection> for ApiError {
    fn from(err: QueryRejection) -> Self {
        let reference = Uuid::new_v4();
        warn!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        ApiError::BadRequest(ApiErrorResponse {
            description: err.to_string(),
            reference,
        })
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        ApiError::InternalServerError(ApiErrorResponse {
            description: err.to_string(),
            reference,
        })
    }
}

impl From<models::Error> for ApiError {
    fn from(err: models::Error) -> Self {
        let reference = Uuid::new_v4();
        error!(
            error_reference = reference.to_string(),
            "API server error: {err} {err:?}"
        );

        match err {
            Error::Serialization(_) => ApiError::BadRequest(ApiErrorResponse {
                description: err.to_string(),
                reference,
            }),
            Error::NotFound(_) => ApiError::NotFound(ApiErrorResponse {
                description: "Not found".to_string(),
                reference,
            }),
            Error::ForeignKeyViolation => ApiError::BadRequest(ApiErrorResponse {
                description: "Foreign key violation".to_string(),
                reference,
            }),
            Error::Conflict => ApiError::Conflict(ApiErrorResponse {
                description: "Conflict".to_string(),
                reference,
            }),
            Error::BadRequest(err) => ApiError::BadRequest(ApiErrorResponse {
                description: err.to_string(),
                reference,
            }),
            Error::TooManyRequests => ApiError::TooManyRequests(ApiErrorResponse {
                description: "Too many requests".to_string(),
                reference,
            }),
            Error::OrgBlocked => ApiError::Forbidden(ApiErrorResponse {
                description:
                    "Your organization is blocked from sending email. Please contact the support"
                        .to_string(),
                reference,
            }),
            _ => ApiError::InternalServerError(ApiErrorResponse {
                description: "Database error".to_string(),
                reference,
            }),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::BadRequest(body) => (StatusCode::BAD_REQUEST, Json(body)),
            ApiError::NotFound(body) => (StatusCode::NOT_FOUND, Json(body)),
            ApiError::Conflict(body) => (StatusCode::CONFLICT, Json(body)),
            ApiError::TooManyRequests(body) => (StatusCode::TOO_MANY_REQUESTS, Json(body)),
            ApiError::InternalServerError(body) => (StatusCode::INTERNAL_SERVER_ERROR, Json(body)),
            ApiError::Forbidden(body) => (StatusCode::FORBIDDEN, Json(body)),
            ApiError::Unauthorized(body) => (StatusCode::UNAUTHORIZED, Json(body)),
            ApiError::PreconditionFailed(body) => (StatusCode::PRECONDITION_FAILED, Json(body)),
            ApiError::BadGateway(body) => (StatusCode::PRECONDITION_FAILED, Json(body)),
            ApiError::PayloadToLarge(body) => (StatusCode::PAYLOAD_TOO_LARGE, Json(body)),
            ApiError::RequestTimeout(body) => (StatusCode::REQUEST_TIMEOUT, Json(body)),
        }
        .into_response()
    }
}
