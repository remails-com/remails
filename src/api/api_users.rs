use crate::{
    api::{
        ApiState,
        error::{AppError, ApiResult},
        validation::ValidatedJson,
        whoami::WhoamiResponse,
    },
    models::{
        ApiUser, ApiUserId, ApiUserRepository, ApiUserUpdate, Error, Password, PasswordUpdate,
        TotpCodeDetails, TotpFinishEnroll, TotpId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use garde::Validate;
use http::header;
use tracing::{debug, info};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(update_user))
        .routes(routes!(update_password, delete_password))
        .routes(routes!(start_enroll_totp, finish_enroll_totp))
        .routes(routes!(totp_codes, delete_totp_code))
}

fn has_read_access(user_id: ApiUserId, user: &ApiUser) -> Result<(), AppError> {
    has_write_access(user_id, user)
}

fn has_write_access(user_id: ApiUserId, user: &ApiUser) -> Result<(), AppError> {
    if *user.id() == user_id {
        return Ok(());
    }
    Err(AppError::Forbidden)
}

/// Update API user details
#[utoipa::path(put, path = "/api_user/{user_id}",
    tags = ["internal", "API users"],
    request_body = ApiUserUpdate,
    responses(
        (status = 200, description = "User successfully updated", body = WhoamiResponse),
        AppError,
))]
pub async fn update_user(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
    ValidatedJson(update): ValidatedJson<ApiUserUpdate>,
) -> ApiResult<WhoamiResponse> {
    has_write_access(user_id, &user)?;

    repo.update(update, &user_id).await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "updated user details"
    );

    Ok(Json(WhoamiResponse::logged_in(
        repo.find_by_id(&user_id)
            .await
            .transpose()
            .ok_or(Error::NotFound("User not found"))??,
    )))
}

/// Update API user password
#[utoipa::path(put, path = "/api_user/{user_id}/password",
    tags = ["internal", "API users"],
    request_body = PasswordUpdate,
    responses(
        (status = 200, description = "User successfully updated"),
        AppError,
))]
pub async fn update_password(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
    ValidatedJson(update): ValidatedJson<PasswordUpdate>,
) -> Result<(), AppError> {
    has_write_access(user_id, &user)?;

    repo.update_password(update, &user_id).await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "updated user password"
    );

    Ok(())
}

#[derive(ToSchema)]
#[schema(format = Binary, value_type = String)]
struct Png(#[schema(inline)] Vec<u8>);

/// Start the TOTP enrollment process
///
/// Returns a PNG image of the QR code to scan with an authenticator app.
/// To finish enrolling, call POST `/api_user/{user_id}/totp/enroll` afterward.
#[utoipa::path(get, path = "/api_user/{user_id}/totp/enroll",
    tags = ["internal", "API users"],
    responses(
        (status = 200, description = "TOTP enrollment successfully started", content_type = "image/png", body = inline(Png)),
        AppError,
))]
pub async fn start_enroll_totp(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
) -> Result<impl IntoResponse, AppError> {
    has_write_access(user_id, &user)?;

    let png = Png(repo.start_enroll_totp(&user_id).await?);

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "started enrolling TOTP"
    );

    let headers = [
        (header::CONTENT_TYPE, "image/png".to_string()),
        (
            header::CACHE_CONTROL,
            "no-cache, no-store, max-age=0, must-revalidate".to_string(),
        ),
        (header::PRAGMA, "no-cache".to_string()),
        (header::EXPIRES, "0".to_string()),
    ];

    Ok((headers, png.0))
}

/// Finish the TOTP enrollment
///
/// To verify that the user saved the secret and can generate code, you have to send the
/// current TOTP together with an optional description
#[utoipa::path(post, path = "/api_user/{user_id}/totp/enroll",
    request_body = TotpFinishEnroll,
    tags = ["internal", "API users"],
    responses(
        (status = 200, description = "TOTP enrollment finished", body = TotpCodeDetails),
        AppError,
))]
pub async fn finish_enroll_totp(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
    ValidatedJson(finish): ValidatedJson<TotpFinishEnroll>,
) -> ApiResult<TotpCodeDetails> {
    has_write_access(user_id, &user)?;

    let code = repo.finish_enroll_totp(&user_id, finish).await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        totp_id = code.id().to_string(),
        "finished enrolling TOTP"
    );

    Ok(Json(code))
}

/// List TOTP codes
#[utoipa::path(get, path = "/api_user/{user_id}/totp",
    tags = ["internal", "API users"],
    responses(
        (status = 200, description = "Successfully fetched active TOTP codes", body = [TotpCodeDetails]),
        AppError,
))]
pub async fn totp_codes(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
) -> ApiResult<Vec<TotpCodeDetails>> {
    has_read_access(user_id, &user)?;

    let codes = repo.totp_codes(&user_id).await?;

    debug!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "retrieved TOTP codes"
    );

    Ok(Json(codes))
}

/// Delete TOTP code
#[utoipa::path(delete, path = "/api_user/{user_id}/totp/{totp_id}",
    tags = ["internal", "API users"],
    responses(
        (status = 200, description = "Successfully deleted TOTP code", body = TotpId),
        AppError,
))]
pub async fn delete_totp_code(
    State(repo): State<ApiUserRepository>,
    Path((user_id, totp_id)): Path<(ApiUserId, TotpId)>,
    user: ApiUser,
) -> ApiResult<TotpId> {
    has_write_access(user_id, &user)?;

    let id = repo.delete_totp(&user_id, &totp_id).await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        totp_id = id.to_string(),
        "deleted TOTP code"
    );

    Ok(Json(id))
}

#[derive(serde::Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct CurrentPassword {
    #[schema(min_length = 10, max_length = 256)]
    #[garde(dive)]
    current_password: Password,
}

/// Delete user password
///
/// This is only allowed if the user has an alternative login method, e.g., via OAuth
#[utoipa::path(delete, path = "/api_user/{user_id}/password",
    request_body = CurrentPassword,
    tags = ["internal", "API users"],
    responses(
        (status = 200, description = "Successfully deleted user password"),
        AppError,
))]
pub async fn delete_password(
    State(repo): State<ApiUserRepository>,
    Path((user_id,)): Path<(ApiUserId,)>,
    user: ApiUser,
    ValidatedJson(update): ValidatedJson<CurrentPassword>,
) -> Result<(), AppError> {
    has_write_access(user_id, &user)?;

    if user.github_user_id().is_none() {
        Err(AppError::PreconditionFailed(
            "You must enable an alternative login method before you can delete your password"
                .to_string(),
        ))?
    }

    repo.delete_password(update.current_password, &user_id)
        .await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "deleted password"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use serde_json::json;
    use sqlx::PgPool;

    use crate::api::{
        tests::{TestServer, deserialize_body, serialize_body},
        whoami::Whoami,
    };

    use super::*;

    impl Whoami {
        fn unwrap_email(&self) -> &str {
            self.email.as_ref().unwrap().as_str()
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_update_user(pool: PgPool) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef"; // not in any organization
        let user_3_id: ApiUserId = user_3.parse().unwrap();
        let users = ApiUserRepository::new(pool.clone());
        let server = TestServer::new(pool, Some(user_3_id)).await;

        // verify starting state
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id.to_string(), user_3);
        assert_eq!(whoami.name, "Test API User 3");
        assert_eq!(whoami.unwrap_email(), "test-api@user-3");
        assert_eq!(whoami.github_id, None);
        assert!(whoami.password_enabled);

        // update user information
        let response = server
            .put(
                format!("/api/api_user/{user_3}"),
                serialize_body(ApiUserUpdate {
                    name: "Updated API User 3".to_string(),
                    email: "updated-api@user-3".parse().unwrap(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id.to_string(), user_3);
        assert_eq!(whoami.name, "Updated API User 3");
        assert_eq!(whoami.unwrap_email(), "updated-api@user-3");
        assert_eq!(whoami.github_id, None);
        assert!(whoami.password_enabled);

        // verify state
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id.to_string(), user_3);
        assert_eq!(whoami.name, "Updated API User 3");
        assert_eq!(whoami.unwrap_email(), "updated-api@user-3");
        assert_eq!(whoami.github_id, None);
        assert!(whoami.password_enabled);

        // current password works
        users
            .check_password(
                &"updated-api@user-3".parse().unwrap(),
                "unsecure123".to_string().into(),
            )
            .await
            .unwrap();

        // update password
        let response = server
            .put(
                format!("/api/api_user/{user_3}/password"),
                // we use json directly here because we don't allow serializing passwords
                serialize_body(json!({
                    "current_password": "unsecure123",
                    "new_password": "new-unsecure-password",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify old password is now incorrect, and new password works
        assert!(matches!(
            users
                .check_password(
                    &"updated-api@user-3".parse().unwrap(),
                    "unsecure123".to_string().into(),
                )
                .await,
            Err(Error::NotFound(_))
        ));
        users
            .check_password(
                &"updated-api@user-3".parse().unwrap(),
                "new-unsecure-password".to_string().into(),
            )
            .await
            .unwrap();

        // can't delete password without alternative login method
        let response = server
            .delete_with_body(
                format!("/api/api_user/{user_3}/password"),
                serialize_body(json!({"current_password": "new-unsecure-password"})),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);

        // can delete password with alternative login method
        users.add_github_id(&user_3_id, 37).await.unwrap();
        let response = server
            .delete_with_body(
                format!("/api/api_user/{user_3}/password"),
                serialize_body(json!({"current_password": "new-unsecure-password"})),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify state
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.id.to_string(), user_3);
        assert_eq!(whoami.name, "Updated API User 3");
        assert_eq!(whoami.unwrap_email(), "updated-api@user-3");
        assert_eq!(whoami.github_id, Some("37".to_string()));
        assert!(!whoami.password_enabled);
    }

    async fn test_update_user_no_access(
        pool: PgPool,
        user: Option<ApiUserId>,
        password: &str,
        user_status_code: StatusCode,
        password_status_code: StatusCode,
    ) {
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef"; // not in any organization
        let user_3_id: ApiUserId = user_3.parse().unwrap();
        let users = ApiUserRepository::new(pool.clone());
        let server = TestServer::new(pool, user).await;

        // can't update user
        let response = server
            .put(
                format!("/api/api_user/{user_3}"),
                serialize_body(ApiUserUpdate {
                    name: "Updated API User 3".to_string(),
                    email: "updated-api@user-3".parse().unwrap(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), user_status_code);

        // verify user remains unchanged
        if !user_status_code.is_success() {
            let user = users.find_by_id(&user_3_id).await.unwrap().unwrap();
            assert_eq!(user.name, "Test API User 3");
            assert_eq!(user.email, Some("test-api@user-3".parse().unwrap()));
        }

        // can't update password
        let response = server
            .put(
                format!("/api/api_user/{user_3}/password"),
                serialize_body(json!({
                    "current_password": password,
                    "new_password": "new-unsecure-password",
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), password_status_code);

        // can't delete password (with alternative login method enabled)
        users.add_github_id(&user_3_id, 37).await.unwrap();
        let response = server
            .delete_with_body(
                format!("/api/api_user/{user_3}/password"),
                serialize_body(json!({"current_password": password})),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), password_status_code);

        // verify password remains unchanged
        let user = users.find_by_id(&user_3_id).await.unwrap().unwrap();
        assert!(user.password_enabled());
        users
            .check_password(&user.email.unwrap(), "unsecure123".to_string().into())
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_update_user_no_access_not_logged_in(pool: PgPool) {
        test_update_user_no_access(
            pool,
            None,
            "unsecure123",
            StatusCode::UNAUTHORIZED,
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_update_user_no_access_wrong_user(pool: PgPool) {
        test_update_user_no_access(
            pool,
            Some("9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap()), // user_1 instead of user_3
            "unsecure123",
            StatusCode::FORBIDDEN,
            StatusCode::FORBIDDEN,
        )
        .await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_update_user_no_access_wrong_password(pool: PgPool) {
        test_update_user_no_access(
            pool,
            Some("54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap()), // user_3
            "wrong-password",
            StatusCode::OK,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
}
