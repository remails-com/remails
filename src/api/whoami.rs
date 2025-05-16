use crate::models::{ApiUser, ApiUserId, ApiUserRole};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use email_address::EmailAddress;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct WhoamiResponse {
    pub id: ApiUserId,
    pub name: String,
    pub roles: Vec<ApiUserRole>,
    pub email: EmailAddress,
    pub github_id: Option<String>,
    pub password_enabled: bool,
}

impl From<ApiUser> for WhoamiResponse {
    fn from(user: ApiUser) -> Self {
        WhoamiResponse {
            id: *user.id(),
            roles: user.roles(),
            github_id: user.github_user_id().map(|id| id.to_string()),
            password_enabled: user.password_enabled(),
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
