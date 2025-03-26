use axum::{Json, extract::State};

use crate::models::{
    ApiUser, SmtpCredential, SmtpCredentialRepository, SmtpCredentialRequest,
    SmtpCredentialResponse,
};

use super::error::{ApiError, ApiResult};

pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
    Json(request): Json<SmtpCredentialRequest>,
) -> ApiResult<SmtpCredentialResponse> {
    if !api_user.is_super_admin() {
        return Err(ApiError::Forbidden);
    }

    let new_credential = repo.generate(&request).await?;

    Ok(Json(new_credential))
}

pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<SmtpCredential>> {
    if !api_user.is_super_admin() {
        return Err(ApiError::Forbidden);
    }

    let credentials = repo.list().await?;

    Ok(Json(credentials))
}
