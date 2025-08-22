use crate::models::{ApiUser, ApiUserId, OrgRole, Role};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use email_address::EmailAddress;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct WhoamiResponse {
    pub id: ApiUserId,
    pub name: String,
    pub global_role: Option<Role>,
    pub org_roles: Vec<OrgRole>,
    pub email: EmailAddress,
    pub github_id: Option<String>,
    pub password_enabled: bool,
}

impl From<ApiUser> for WhoamiResponse {
    fn from(user: ApiUser) -> Self {
        WhoamiResponse {
            id: *user.id(),
            global_role: user.global_role.clone(),
            org_roles: user.org_roles.clone(),
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
