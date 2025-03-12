use crate::api::error::ApiError;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing variable from environment: {0}")]
    MissingEnvironmentVariable(&'static str),
    #[error("oauth token {0}")]
    OauthToken(String),
    #[error("fetching github user {0}")]
    FetchUser(String),
    #[error("parsing github user {0}")]
    ParseUser(String),
    #[error("json {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed deserializing user {0}")]
    DeserializeUser(serde_json::Error),
    #[error("missing csrf cookie")]
    MissingCSRFCookie,
    #[error("the CSRF token did not match")]
    CSRFTokenMismatch,
    #[error("invalid state")]
    ServiceNotFound,
}

impl Error {
    pub fn user_message(&self) -> String {
        match self {
            Self::MissingEnvironmentVariable(name) => {
                format!("Missing environment variable: {}", name)
            }
            Self::OauthToken(_) => "Error fetching OAuth token".to_string(),
            Self::FetchUser(_) => "An error occurred while fetching the GitHub user".to_string(),
            Self::ParseUser(_) => "An error occurred while parsing the GitHub user".to_string(),
            Self::Json(_) => "An error occurred while processing JSON".to_string(),
            Self::DeserializeUser(_) => {
                "An error occurred while deserializing the user".to_string()
            }
            Self::MissingCSRFCookie => "Missing CSRF cookie".to_string(),
            Self::CSRFTokenMismatch => "The CSRF token did not match".to_string(),
            Self::ServiceNotFound => "Service not found".to_string(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::MissingEnvironmentVariable(_)
            | Error::Json(_)
            | Error::DeserializeUser(_)
            | Error::ServiceNotFound => StatusCode::INTERNAL_SERVER_ERROR,

            Error::FetchUser(_) | Error::ParseUser(_) => StatusCode::BAD_GATEWAY,

            Error::OauthToken(_) | Error::MissingCSRFCookie | Error::CSRFTokenMismatch => {
                StatusCode::UNAUTHORIZED
            }
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
