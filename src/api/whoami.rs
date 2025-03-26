use crate::models::{ApiUser, ApiUserRole};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct WhoamiResponse {
    pub roles: Vec<ApiUserRole>,
    pub email: String,
}

impl From<ApiUser> for WhoamiResponse {
    fn from(user: ApiUser) -> Self {
        WhoamiResponse {
            roles: user.roles(),
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
