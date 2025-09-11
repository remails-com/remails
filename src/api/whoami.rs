use crate::{
    api::auth::MfaPending,
    models::{ApiUser, ApiUserId, OrgRole, Role},
};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use email_address::EmailAddress;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Whoami {
    pub id: ApiUserId,
    pub name: String,
    pub global_role: Option<Role>,
    pub org_roles: Vec<OrgRole>,
    pub email: EmailAddress,
    pub github_id: Option<String>,
    pub password_enabled: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "login_status", rename_all = "snake_case")]
pub enum WhoamiResponse {
    LoggedIn(Whoami),
    MfaPending,
}

impl WhoamiResponse {
    pub fn logged_in(user: ApiUser) -> Self {
        Self::LoggedIn(Whoami {
            id: *user.id(),
            global_role: user.global_role,
            org_roles: user.org_roles.clone(),
            github_id: user.github_user_id().map(|id| id.to_string()),
            password_enabled: user.password_enabled(),
            name: user.name,
            email: user.email,
        })
    }

    /// Panics if whoami response is not logged in
    #[cfg(test)]
    pub fn unwrap_logged_in(&self) -> &Whoami {
        match self {
            WhoamiResponse::LoggedIn(whoami) => whoami,
            WhoamiResponse::MfaPending => panic!("Unexpected MFA pending"),
        }
    }
}

pub async fn whoami(user: Option<ApiUser>, mfa_pending: Option<MfaPending>) -> Response {
    match (user, mfa_pending) {
        (Some(user), None) => Json(WhoamiResponse::logged_in(user)).into_response(),
        (None, Some(_)) => Json(WhoamiResponse::MfaPending).into_response(),
        (None, None) => Json(json!({"error": "Not logged in"})).into_response(),
        (Some(_), Some(_)) => {
            debug_assert!(
                false,
                "Logged in user and MFA pending should not be possible at the same time."
            );
            Json(json!({"error": "Not logged in"})).into_response()
        }
    }
}
