use axum::{Json, extract::State};
use serde::Deserialize;

use crate::smtp_credential::{SmtmCredential, SmtpCredentialRepository};

use super::{
    auth::ApiUser,
    error::{ApiError, ApiResult},
};

#[derive(Debug, Deserialize)]
pub struct NewSmtpCredential {
    username: String,
    password: String,
}

pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
    Json(NewSmtpCredential { username, password }): Json<NewSmtpCredential>,
) -> ApiResult<SmtmCredential> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let new_credential = SmtmCredential::new(username, password);
    let credential = repo.insert(&new_credential).await?;

    Ok(Json(credential))
}

pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<SmtmCredential>> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let credentials = repo.list().await?;

    Ok(Json(credentials))
}
