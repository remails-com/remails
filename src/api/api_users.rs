use crate::{
    api::{
        error::{ApiError, ApiResult},
        validation::ValidatedJson,
        whoami::WhoamiResponse,
    },
    models::{
        ApiUser, ApiUserId, ApiUserRepository, ApiUserUpdate, Error, Password, PasswordUpdate,
        TotpCode, TotpFinishEnroll, TotpId,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::header;
use tracing::{debug, info};

fn has_read_access(user_id: ApiUserId, user: &ApiUser) -> Result<(), ApiError> {
    has_write_access(user_id, user)
}

fn has_write_access(user_id: ApiUserId, user: &ApiUser) -> Result<(), ApiError> {
    if *user.id() == user_id {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn update_user(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<ApiUserUpdate>,
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

pub async fn update_password(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<PasswordUpdate>,
) -> Result<(), ApiError> {
    has_write_access(user_id, &user)?;

    repo.update_password(update, &user_id).await?;

    info!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "updated user password"
    );

    Ok(())
}

pub async fn start_enroll_totp(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
) -> Result<impl IntoResponse, ApiError> {
    has_write_access(user_id, &user)?;

    let png = repo.start_enroll_totp(&user_id).await?;

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

    Ok((headers, png))
}

pub async fn finish_enroll_totp(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    ValidatedJson(finish): ValidatedJson<TotpFinishEnroll>,
) -> ApiResult<TotpCode> {
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

pub async fn totp_codes(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
) -> ApiResult<Vec<TotpCode>> {
    has_read_access(user_id, &user)?;

    let codes = repo.totp_codes(&user_id).await?;

    debug!(
        user_id = user_id.to_string(),
        executing_user_id = user.id().to_string(),
        "retrieved TOTP codes"
    );

    Ok(Json(codes))
}

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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentPassword {
    current_password: Password,
}

pub async fn delete_password(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<CurrentPassword>,
) -> Result<(), ApiError> {
    has_write_access(user_id, &user)?;

    if user.github_user_id().is_none() {
        Err(ApiError::PreconditionFailed(
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
                "unsecure".to_string().into(),
            )
            .await
            .unwrap();

        // update password
        let response = server
            .put(
                format!("/api/api_user/{user_3}/password"),
                // we use json directly here because we don't allow serializing passwords
                serialize_body(json!({
                    "current_password": "unsecure",
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
                    "unsecure".to_string().into(),
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
            .check_password(&user.email.unwrap(), "unsecure".to_string().into())
            .await
            .unwrap();
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_update_user_no_access_not_logged_in(pool: PgPool) {
        test_update_user_no_access(
            pool,
            None,
            "unsecure",
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
            "unsecure",
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
