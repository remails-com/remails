use crate::models::{ApiUser, ApiUserRole};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use email_address::EmailAddress;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct WhoamiResponse {
    pub name: String,
    pub roles: Vec<ApiUserRole>,
    pub email: EmailAddress,
}

impl From<ApiUser> for WhoamiResponse {
    fn from(user: ApiUser) -> Self {
        WhoamiResponse {
            roles: user.roles(),
            name: user.name,
            email: user.email,
        }
    }
}

pub async fn whoami(user: Option<ApiUser>) -> Response {
    match user {
        Some(user) => Json(WhoamiResponse::from(user)).into_response(),
        None => Json(json!({"error": "Not logged in"})).into_response(),
    }
}
