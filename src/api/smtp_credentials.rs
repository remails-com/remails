use super::error::{AppError, ApiResult};
use crate::{
    api::{ApiState, auth::Authenticated, validation::ValidatedJson},
    models::{
        OrganizationId, ProjectId, SmtpCredential, SmtpCredentialId, SmtpCredentialRepository,
        SmtpCredentialRequest, SmtpCredentialResponse, SmtpCredentialUpdateRequest, StreamId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(create_smtp_credential, list_smtp_credential))
        .routes(routes!(update_smtp_credential, remove_smtp_credential))
}

/// Create a new SMTP credential
#[utoipa::path(post, path = "/organizations/{org_id}/projects/{proj_id}/streams/{stream_id}/smtp_credentials",
    tags = ["SMTP Credentials"],
    request_body = SmtpCredentialRequest,
    responses(
        (status = 201, description = "Successfully created SMTP credential", body = SmtpCredentialResponse),
        AppError,
    )
)]
pub async fn create_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    ValidatedJson(request): ValidatedJson<SmtpCredentialRequest>,
) -> Result<impl IntoResponse, AppError> {
    user.has_org_write_access(&org_id)?;

    let new_credential = repo.generate(org_id, proj_id, stream_id, &request).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = new_credential.id().to_string(),
        credential_username = new_credential.username(),
        "created SMTP credential"
    );

    Ok((StatusCode::CREATED, Json(new_credential)))
}

/// Update an SMTP credential
#[utoipa::path(put, path = "/organizations/{org_id}/projects/{proj_id}/streams/{stream_id}/smtp_credentials/{credential_id}",
    tags = ["SMTP Credentials"],
    request_body = SmtpCredentialUpdateRequest,
    responses(
        (status = 200, description = "Successfully updated SMTP credential", body = SmtpCredential),
        AppError,
    )
)]
pub async fn update_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id, stream_id, credential_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        SmtpCredentialId,
    )>,
    ValidatedJson(request): ValidatedJson<SmtpCredentialUpdateRequest>,
) -> ApiResult<SmtpCredential> {
    user.has_org_write_access(&org_id)?;

    let update = repo
        .update(org_id, proj_id, stream_id, credential_id, &request)
        .await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = update.id().to_string(),
        credential_username = update.username(),
        "updated SMTP credential"
    );

    Ok(Json(update))
}

/// List all active SMTP credentials
#[utoipa::path(get, path = "/organizations/{org_id}/projects/{proj_id}/streams/{stream_id}/smtp_credentials",
    tags = ["SMTP Credentials"],
    responses(
        (status = 200, description = "Successfully fetched SMTP credentials", body = [SmtpCredential]),
        AppError,
    )
)]
pub async fn list_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    user: Box<dyn Authenticated>,
) -> ApiResult<Vec<SmtpCredential>> {
    user.has_org_read_access(&org_id)?;

    let credentials = repo.list(org_id, proj_id, stream_id).await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        "listed {} SMTP credentials",
        credentials.len()
    );

    Ok(Json(credentials))
}

/// Delete an SMTP credential
#[utoipa::path(delete, path = "/organizations/{org_id}/projects/{proj_id}/streams/{stream_id}/smtp_credentials/{credential_id}",
    tags = ["SMTP Credentials"],
    responses(
        (status = 200, description = "Successfully delete SMTP credential", body = SmtpCredentialId),
        AppError,
    )
)]
pub async fn remove_smtp_credential(
    State(repo): State<SmtpCredentialRepository>,
    Path((org_id, proj_id, stream_id, credential_id)): Path<(
        OrganizationId,
        ProjectId,
        StreamId,
        SmtpCredentialId,
    )>,
    user: Box<dyn Authenticated>,
) -> ApiResult<SmtpCredentialId> {
    user.has_org_write_access(&org_id)?;

    let credential_id = repo
        .remove(org_id, proj_id, stream_id, credential_id)
        .await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        credential_id = credential_id.to_string(),
        "deleted SMTP credential",
    );

    Ok(Json(credential_id))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::SmtpCredentialResponse,
    };

    use super::*;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams",)
    ))]
    async fn test_smtp_credential_lifecycle(pool: PgPool) {
        let user_1 = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1
        let server = TestServer::new(pool.clone(), Some(user_1)).await;

        // start with no credentials
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let credentials: Vec<SmtpCredential> = deserialize_body(response.into_body()).await;
        assert_eq!(credentials.len(), 0);

        // create a credential
        let new_cred = SmtpCredentialRequest {
            description: "Test Credential".to_string(),
            username: "testuser".to_string(),
        };
        let response = server
            .post(
                format!(
                    "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
                ),
                serialize_body(&new_cred),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_credential: SmtpCredentialResponse =
            deserialize_body(response.into_body()).await;
        assert_eq!(created_credential.description(), new_cred.description);
        assert_eq!(created_credential.stream_id().to_string(), stream_1);
        assert!(created_credential.username().ends_with(&new_cred.username));

        // list credentials
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let credentials: Vec<SmtpCredential> = deserialize_body(response.into_body()).await;
        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].id(), created_credential.id());
        assert_eq!(
            credentials[0].description(),
            created_credential.description()
        );

        // update credential
        let updated_cred = SmtpCredentialUpdateRequest {
            description: "Updated Credential".to_string(),
        };
        let response = server
            .put(
                format!(
                    "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials/{}",
                    created_credential.id()
                ),
                serialize_body(&updated_cred),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let credential: SmtpCredential = deserialize_body(response.into_body()).await;
        assert_eq!(credential.description(), updated_cred.description);
        assert_eq!(credential.id(), created_credential.id());

        // list credentials
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let credentials: Vec<SmtpCredential> = deserialize_body(response.into_body()).await;
        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].id(), created_credential.id());
        assert_eq!(credentials[0].description(), updated_cred.description);

        // remove credential
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials/{}",
                created_credential.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let deleted_id: SmtpCredentialId = deserialize_body(response.into_body()).await;
        assert_eq!(deleted_id, created_credential.id());

        // check if credential is deleted
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let credentials: Vec<SmtpCredential> = deserialize_body(response.into_body()).await;
        assert_eq!(credentials.len(), 0);
    }

    async fn test_smtp_credential_no_access(
        server: TestServer,
        read_status_code: StatusCode,
        write_status_code: StatusCode,
    ) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1
        let cred_1 = "9442cbbf-9897-4af7-9766-4ac9c1bf49cf"; // credential in stream 1

        // can't list credentials
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), read_status_code);

        // can't create credentials
        let response = server
            .post(
                format!(
                    "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials"
                ),
                serialize_body(&SmtpCredentialRequest {
                    description: "Test Credential".to_string(),
                    username: "testuser".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't update credentials
        let response = server
            .put(
                format!(
                    "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials/{cred_1}"
                ),
                serialize_body(&SmtpCredentialUpdateRequest {
                    description: "Updated Credential".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);

        // can't delete credentials
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}/smtp_credentials/{cred_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), write_status_code);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_smtp_credential_no_access_wrong_user(pool: PgPool) {
        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let server = TestServer::new(pool.clone(), Some(user_2)).await;
        test_smtp_credential_no_access(server, StatusCode::FORBIDDEN, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_smtp_credential_no_access_read_only(pool: PgPool) {
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e".parse().unwrap(); // is read only in org 1
        let server = TestServer::new(pool.clone(), Some(user_5)).await;
        test_smtp_credential_no_access(server, StatusCode::OK, StatusCode::FORBIDDEN).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "api_users",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn test_smtp_credential_no_access_not_logged_in(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;
        test_smtp_credential_no_access(server, StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED)
            .await;
    }
}
