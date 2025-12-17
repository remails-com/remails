use crate::{
    api::{ApiState, auth::MfaPending},
    models::{ApiUser, ApiUserId, OrgRole, Role},
};
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::Serialize;
use serde_json::json;
use utoipa::{
    ToSchema,
    openapi::{Object, ObjectBuilder},
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new().routes(routes!(whoami))
}

fn email_openapi_schema() -> Object {
    ObjectBuilder::new()
        .schema_type(utoipa::openapi::schema::Type::String)
        .format(Some(utoipa::openapi::SchemaFormat::Custom(
            "email".to_string(),
        )))
        .description(Some(
            "Logged-in session users always have an email, but API keys do not",
        ))
        .build()
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Whoami {
    pub id: ApiUserId,
    pub name: String,
    /// Logged-in session users always have an email, but API keys do not
    #[schema(schema_with = email_openapi_schema)]
    pub email: Option<EmailAddress>,
    pub global_role: Option<Role>,
    pub org_roles: Vec<OrgRole>,
    /// Unlike in `ApiUser`, here the GitHub ID is a string
    pub github_id: Option<String>,
    pub password_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ApiUser> for Whoami {
    fn from(user: ApiUser) -> Self {
        Self {
            id: *user.id(),
            global_role: user.global_role,
            github_id: user.github_user_id().map(|id| id.to_string()),
            password_enabled: user.password_enabled(),
            org_roles: user.org_roles,
            name: user.name,
            email: user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "login_status", rename_all = "snake_case")]
pub enum WhoamiResponse {
    LoggedIn(Whoami),
    MfaPending,
}

impl WhoamiResponse {
    pub fn logged_in(user: ApiUser) -> Self {
        Self::LoggedIn(user.into())
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

/// Whoami endpoint
///
/// Returns information about the currently logged-in user or API key used
#[utoipa::path(get, path = "/whoami",
    tags = ["Misc"],
    responses(
        (status = 200, description = "Current authentication status", body = WhoamiResponse)
    )
)]
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
