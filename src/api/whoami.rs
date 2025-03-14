use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;

use super::auth::{ApiUser, Role};

#[derive(Debug, Clone, Serialize)]
pub struct WhoamiResponse {
    pub role: Role,
    pub email: String,
}

impl From<ApiUser> for WhoamiResponse {
    fn from(user: ApiUser) -> Self {
        Self {
            email: user.email,
            role: user.role,
        }
    }
}

pub async fn whoami(user: Option<ApiUser>) -> Response {
    match user {
        Some(user) => Json(WhoamiResponse::from(user)).into_response(),
        None => Json(json!({"error": "Not logged in"})).into_response(),
    }
}
