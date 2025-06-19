use super::error::{ApiError, ApiResult};
use crate::models::{
    ApiUser, OrganizationId, ProjectId, SmtpCredential, SmtpCredentialId, SmtpCredentialRepository,
    SmtpCredentialRequest, SmtpCredentialUpdateRequest, StreamId,
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

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
    if user.org_admin().contains(&org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    user: ApiUser,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    Json(request): Json<SmtpCredentialRequest>,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(org_id, proj_id, stream_id, None, &user)?;

    let new_credential = repo.generate(org_id, proj_id, stream_id, &request).await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = new_credential.id().to_string(),
        credential_username = new_credential.username(),
        "created SMTP credential"
    );

    Ok((StatusCode::CREATED, Json(new_credential)))
}

pub async fn update_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    user: ApiUser,
    Path((org_id, proj_id, stream_id, cred_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        SmtpCredentialId,
    )>,
    Json(request): Json<SmtpCredentialUpdateRequest>,
) -> ApiResult<SmtpCredential> {
    has_write_access(org_id, proj_id, stream_id, None, &user)?;

    let update = repo
        .update(org_id, proj_id, stream_id, cred_id, &request)
        .await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = update.id().to_string(),
        credential_username = update.username(),
        "updated SMTP credential"
    );

    Ok(Json(update))
}

pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    user: ApiUser,
) -> ApiResult<Vec<SmtpCredential>> {
    has_read_access(org_id, proj_id, stream_id, None, &user)?;

    let credentials = repo.list(org_id, proj_id, stream_id).await?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        "listed {} SMTP credentials",
        credentials.len()
    );

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

    let credential_id = repo
        .remove(org_id, proj_id, stream_id, credential_id)
        .await?;

    info!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = credential_id.to_string(),
        "deleted SMTP credential",
    );

    Ok(Json(credential_id))
}
