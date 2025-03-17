use axum::{Json, extract::State};
use serde::Deserialize;

use crate::models::{SmtpCredential, SmtpCredentialRepository};

use super::{
    auth::ApiUser,
    error::{ApiError, ApiResult},
};

#[derive(Debug, Deserialize)]
pub struct NewSmtpCredential {
    username: String,
    password: String,
    domain: String,
}

pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
    Json(NewSmtpCredential {
        username,
        password,
        domain,
    }): Json<NewSmtpCredential>,
) -> ApiResult<SmtpCredential> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let new_credential = SmtpCredential::new(username, password, domain);
    let credential = repo.create(&new_credential).await?;

    Ok(Json(credential))
}

pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<SmtpCredential>> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let credentials = repo.list().await?;

    Ok(Json(credentials))
}
