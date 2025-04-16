use crate::models::{
    ApiUser, OrganizationId, ProjectId, SmtpCredential, SmtpCredentialId, SmtpCredentialRepository,
    SmtpCredentialRequest, SmtpCredentialResponse, StreamId,
};
use axum::{
    Json,
    extract::{Path, State},
};

use super::error::{ApiError, ApiResult};

fn has_read_access(
    org: OrganizationId,
    proj: ProjectId,
    stream_id: StreamId,
    smtp_cred: Option<SmtpCredentialId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    has_write_access(org, proj, stream_id, smtp_cred, user)
}

fn has_write_access(
    org: OrganizationId,
    _proj: ProjectId,
    _stream_id: StreamId,
    _smtp_cred: Option<SmtpCredentialId>,
    user: &ApiUser,
) -> Result<(), ApiError> {
    if user.org_admin().iter().any(|o| *o == org) {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    user: ApiUser,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    Json(request): Json<SmtpCredentialRequest>,
) -> ApiResult<SmtpCredentialResponse> {
    has_write_access(org_id, proj_id, stream_id, None, &user)?;

    let new_credential = repo.generate(org_id, proj_id, stream_id, &request).await?;

    Ok(Json(new_credential))
}

pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    user: ApiUser,
) -> ApiResult<Vec<SmtpCredential>> {
    has_read_access(org_id, proj_id, stream_id, None, &user)?;

    let credentials = repo.list(org_id, proj_id, stream_id).await?;

    Ok(Json(credentials))
}

pub async fn remove_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    Path((org_id, proj_id, stream_id, credential_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        SmtpCredentialId,
    )>,
    user: ApiUser,
) -> ApiResult<SmtpCredentialId> {
    has_write_access(org_id, proj_id, stream_id, Some(credential_id), &user)?;

    let credentials = repo
        .remove(org_id, proj_id, stream_id, credential_id)
        .await?;

    Ok(Json(credentials))
}
