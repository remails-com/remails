use crate::{
    api::{
        ApiState,
        error::{ApiResult, AppError},
        validation::ValidatedJson,
        whoami::WhoamiResponse,
    },
    models::{
        ApiUser, ApiUserId, ApiUserRepository, ApiUserUpdate, Error, Password, PasswordUpdate,
        PwResetId, ResetLinkCheck, TotpCode, TotpCodeDetails, TotpFinishEnroll, TotpId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use garde::Validate;
use http::header;
use serde::Deserialize;
use tracing::{debug, info};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(update_user))
        .routes(routes!(is_password_reset_active))
        .routes(routes!(password_reset))
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

/// Check password reset link
///
/// Check if the password reset link is still active and weather 2FA is active for that account
#[utoipa::path(get, path = "/login/password/reset/{pw_reset_id}",
    tags = ["internal", "API users"],
    security(()),
    responses(
        (status = 200, description = "password reset link still valid", body = ResetLinkCheck),
        AppError,
))]
pub async fn is_password_reset_active(
    State(repo): State<ApiUserRepository>,
    Path((pw_reset_id,)): Path<(PwResetId,)>,
) -> ApiResult<ResetLinkCheck> {
    let active = repo.is_password_reset_active(pw_reset_id).await?;

    info!(
        pw_reset_id = pw_reset_id.to_string(),
        ?active,
        "queried if password reset link is active"
    );

    Ok(Json(active))
}

#[derive(Deserialize, Validate, ToSchema)]
struct PasswordReset {
    #[garde(dive)]
    reset_secret: Password,
    #[garde(dive)]
    new_password: Password,
    #[garde(dive)]
    totp_code: Option<TotpCode>,
}

/// Reset password
///
/// Set new password using the password reset secret that was sent by mail.
/// If the user has 2FA activated, they must also provide a valid TOTP code
#[utoipa::path(post, path = "/login/password/reset/{pw_reset_id}",
    tags = ["internal", "API users"],
    request_body = PasswordReset,
    security(()),
    responses(
        (status = 200, description = "password successfully set"),
        AppError,
))]
async fn password_reset(
    State(repo): State<ApiUserRepository>,
    Path((pw_reset_id,)): Path<(PwResetId,)>,
    ValidatedJson(req): ValidatedJson<PasswordReset>,
) -> Result<(), AppError> {
    repo.finish_password_reset(
        pw_reset_id,
        req.reset_secret,
        req.new_password,
        req.totp_code,
    )
    .await?;

    info!(
        pw_reset_id = pw_reset_id.to_string(),
        "set new password using reset link"
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
    #[schema(min_length = 6, max_length = 256)]
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
    use super::*;
    use crate::api::{
        auth::tests::get_session_cookie,
        tests::{TestServer, deserialize_body, serialize_body},
        whoami::Whoami,
    };
    use http::StatusCode;
    use mail_parser::MessageParser;
    use regex::Regex;
    use serde_json::json;
    use sqlx::PgPool;

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

    #[sqlx::test]
    async fn test_invalid_rw_reset_link(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        let res = server
            .get("/api/login/password/reset/4f6c6024-f1f0-468d-8166-a0824d26e86f")
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let status: ResetLinkCheck = deserialize_body(res.into_body()).await;
        assert_eq!(status, ResetLinkCheck::NotActive);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "runtime_config",)
    ))]
    async fn test_password_reset(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        let res = server
            .post(
                "/api/login/password/reset",
                serialize_body(json! {"test-api@user-2"}),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let raw_data = sqlx::query_scalar!(
            r#"
            SELECT raw_data FROM messages WHERE recipients = '{"test-api@user-2"}'
            "#
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let message = MessageParser::default().parse(&raw_data).unwrap();
        let message = message.body_text(0).unwrap().to_string();

        let regex = Regex::new(r#"https://[^/]*/([^\s#]*)#([^\s)]*)"#).unwrap();
        let captures = regex.captures(message.as_str()).unwrap();
        let reset_link = captures.get(1).unwrap().as_str();
        let reset_secret = captures.get(2).unwrap().as_str();

        // Check reset link status
        let res = server.get(format!("/api/{reset_link}")).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let status: ResetLinkCheck = deserialize_body(res.into_body()).await;
        assert_eq!(status, ResetLinkCheck::ActiveWithout2Fa);

        // Try invalid reset secret
        let res = server
            .post(
                format!("/api/{reset_link}"),
                serialize_body(json!({
                    "new_password": "thisismynewpassword",
                    "reset_secret": "invalidsecret"
                    }
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        // Set new password
        let res = server
            .post(
                format!("/api/{reset_link}"),
                serialize_body(json!({
                    "new_password": "thisismynewpassword",
                    "reset_secret": reset_secret
                    }
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login with new password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-api@user-2",
                    "password": "thisismynewpassword"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = get_session_cookie(response);
    }

    #[sqlx::test]
    async fn test_password_reset_request_returns_ok_if_mail_does_not_exist(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        let res = server
            .post(
                "/api/login/password/reset",
                serialize_body(json! {"does-not-exist@email.com"}),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "runtime_config",)
    ))]
    async fn test_password_reset_with_active_2fa(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        let res = server
            .post(
                "/api/login/password/reset",
                serialize_body(json! {"test-totp-rate-limit@user-4"}),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let raw_data = sqlx::query_scalar!(
            r#"
            SELECT raw_data FROM messages WHERE recipients = '{"test-totp-rate-limit@user-4"}'
            "#
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let message = MessageParser::default().parse(&raw_data).unwrap();
        let message = message.body_text(0).unwrap().to_string();

        let regex = Regex::new(r#"https://[^/]*/([^\s#]*)#([^\s)]*)"#).unwrap();
        let captures = regex.captures(message.as_str()).unwrap();
        let reset_link = captures.get(1).unwrap().as_str();
        let reset_secret = captures.get(2).unwrap().as_str();

        // Check reset link status
        let res = server.get(format!("/api/{reset_link}")).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let status: ResetLinkCheck = deserialize_body(res.into_body()).await;
        assert_eq!(status, ResetLinkCheck::ActiveWith2Fa);

        // Make sure TOTP is required
        let res = server
            .post(
                format!("/api/{reset_link}"),
                serialize_body(json!({
                    "new_password": "thisismynewpassword",
                    "reset_secret": reset_secret,
                    }
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // try invalid totp code
        let res = server
            .post(
                format!("/api/{reset_link}"),
                serialize_body(json!({
                    "new_password": "thisismynewpassword",
                    "reset_secret": reset_secret,
                    "totp_code": "123456"
                    }
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let totp_url = sqlx::query_scalar!(
            r#"
            SELECT url FROM totp WHERE id = '448f8b7c-e6b9-4038-ab73-bc35826fd5da'
            "#
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let totp_code = totp_rs::TOTP::from_url(totp_url)
            .unwrap()
            .generate_current()
            .unwrap();

        // Set new password
        let res = server
            .post(
                format!("/api/{reset_link}"),
                serialize_body(json!({
                    "new_password": "thisismynewpassword",
                    "reset_secret": reset_secret,
                    "totp_code": totp_code
                    }
                )),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login with new password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp-rate-limit@user-4",
                    "password": "thisismynewpassword"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = get_session_cookie(response);
    }
}
