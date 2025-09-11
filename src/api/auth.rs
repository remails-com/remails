use crate::{
    api::{ApiState, error::ApiError, whoami::WhoamiResponse},
    models::{ApiUser, ApiUserId, ApiUserRepository, NewApiUser, OrganizationId, Password, Role},
};
use axum::{
    Json,
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, IntoResponseParts, Redirect, Response, ResponseParts},
};
#[cfg(not(test))]
use axum::{RequestPartsExt, extract::ConnectInfo};
use axum_extra::extract::PrivateCookieJar;
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
#[cfg(not(test))]
use std::net::SocketAddr;
#[cfg(not(test))]
use tracing::error;
use tracing::{debug, trace, warn};

static SESSION_COOKIE_NAME: &str = "SESSION";

impl ApiUser {
    pub fn is_super_admin(&self) -> bool {
        self.global_role
            .as_ref()
            .is_some_and(|role| *role == Role::Admin)
    }

    fn is_at_least(&self, org_id: &OrganizationId, role: Role) -> bool {
        self.org_roles
            .iter()
            .any(|org_role| org_role.org_id == *org_id && org_role.role.is_at_least(role))
    }

    pub fn has_org_read_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        if self.is_at_least(org_id, Role::ReadOnly) || self.is_super_admin() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn has_org_write_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        if self.is_at_least(org_id, Role::Maintainer) || self.is_super_admin() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn has_org_admin_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        if self.is_at_least(org_id, Role::Admin) || self.is_super_admin() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn viewable_organizations(&self) -> Vec<uuid::Uuid> {
        self.org_roles
            .iter()
            .map(|org_role| *org_role.org_id)
            .collect()
    }
}

pub(super) struct SecureCookieStorage {
    jar: PrivateCookieJar,
}

impl FromRequestParts<ApiState> for SecureCookieStorage {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        let jar = PrivateCookieJar::from_headers(&parts.headers, state.config.session_key.clone());

        Ok(Self { jar })
    }
}

impl IntoResponseParts for SecureCookieStorage {
    type Error = Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.jar.into_response_parts(res)
    }
}

impl SecureCookieStorage {
    pub fn remove<C>(self, cookie: C) -> SecureCookieStorage
    where
        C: Into<Cookie<'static>>,
    {
        Self {
            jar: self.jar.remove(cookie),
        }
    }

    pub fn add<C>(self, cookie: C) -> SecureCookieStorage
    where
        C: Into<Cookie<'static>>,
    {
        Self {
            jar: self.jar.add(cookie),
        }
    }

    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.jar.get(name)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LoginState {
    MfaPending,
    LoggedIn,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserCookie {
    id: ApiUserId,
    state: LoginState,
    expires_at: DateTime<Utc>,
}

impl UserCookie {
    pub fn id(&self) -> &ApiUserId {
        &self.id
    }
    pub fn expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }
}

impl UserCookie {
    fn from_api_user(user: &ApiUser, login_state: LoginState) -> Self {
        Self {
            id: *user.id(),
            state: login_state,
            expires_at: Utc::now() + Duration::days(7),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct PasswordLogin {
    email: EmailAddress,
    password: Password,
}

pub(super) async fn password_login(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    Json(login_attempt): Json<PasswordLogin>,
) -> Result<Response, ApiError> {
    repo.check_password(&login_attempt.email, login_attempt.password)
        .await?;
    let user = repo
        .find_by_email(&login_attempt.email)
        .await?
        .ok_or(ApiError::NotFound)?;

    let whoami;
    if repo.mfa_enabled(user.id()).await? {
        cookie_storage = login(&user, LoginState::MfaPending, cookie_storage)?;
        whoami = WhoamiResponse::MfaPending;
    } else {
        cookie_storage = login(&user, LoginState::LoggedIn, cookie_storage)?;
        whoami = WhoamiResponse::logged_in(user);
    }

    Ok((StatusCode::OK, cookie_storage, Json(whoami)).into_response())
}

pub(super) async fn totp_login(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    user: MfaPending,
    Json(totp_code): Json<String>,
) -> Result<Response, ApiError> {
    if !repo.check_totp_code(&user.id(), totp_code.as_str()).await? {
        return Ok((StatusCode::UNAUTHORIZED, Json(WhoamiResponse::MfaPending)).into_response());
    }

    let user = repo
        .find_by_id(&user.id())
        .await?
        .ok_or(ApiError::Unauthorized)?;
    cookie_storage = login(&user, LoginState::LoggedIn, cookie_storage)?;

    Ok((
        StatusCode::OK,
        cookie_storage,
        Json(WhoamiResponse::logged_in(user)),
    )
        .into_response())
}

#[derive(Deserialize)]
pub(super) struct PasswordRegister {
    name: String,
    email: EmailAddress,
    password: Password,
}

pub(super) async fn password_register(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    Json(register_attempt): Json<PasswordRegister>,
) -> Result<Response, ApiError> {
    let new = NewApiUser {
        email: register_attempt.email,
        name: register_attempt.name.trim().to_string(),
        password: Some(register_attempt.password),
        global_role: None,
        org_roles: vec![],
        github_user_id: None,
    };

    let user = repo.create(new).await?;

    cookie_storage = login(&user, LoginState::LoggedIn, cookie_storage)?;
    let whoami = WhoamiResponse::logged_in(user);
    Ok((StatusCode::CREATED, cookie_storage, Json(whoami)).into_response())
}

pub(super) fn login(
    user: &ApiUser,
    login_state: LoginState,
    cookie_storage: SecureCookieStorage,
) -> Result<SecureCookieStorage, serde_json::Error> {
    // Serialize the user data as a string
    let cookie = UserCookie::from_api_user(user, login_state);
    let session_cookie_value = serde_json::to_string(&cookie)?;

    // Create a new session cookie
    let mut session_cookie = Cookie::new(SESSION_COOKIE_NAME, session_cookie_value);
    session_cookie.set_http_only(true);
    session_cookie.set_secure(true);
    session_cookie.set_same_site(SameSite::Lax);
    session_cookie.set_max_age(cookie::time::Duration::days(7));
    session_cookie.set_path("/");

    Ok(cookie_storage.add(session_cookie))
}

pub(super) async fn logout(storage: SecureCookieStorage) -> impl IntoResponse {
    let mut jar = storage.jar;

    // Remove the session cookie from the cookie jar
    if let Some(mut cookie) = jar.get(SESSION_COOKIE_NAME) {
        // Set cookie attributes (necessary for removal) and remove it from the jar
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Lax);
        cookie.set_path("/");

        jar = jar.remove(cookie);
    }

    (jar, Redirect::to("/"))
}

pub struct MfaPending(ApiUserId);

impl MfaPending {
    pub fn id(&self) -> ApiUserId {
        self.0
    }
}

impl<S> FromRequestParts<S> for MfaPending
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiUserRepository: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let api_state: ApiState = FromRef::from_ref(state);
        let jar = PrivateCookieJar::from_headers(&parts.headers, api_state.config.session_key);

        let session_cookie = jar.get(SESSION_COOKIE_NAME).ok_or(ApiError::Unauthorized)?;

        match serde_json::from_str::<UserCookie>(session_cookie.value()) {
            Ok(cookie) => {
                if cookie.expires_at() < &Utc::now() {
                    warn!(
                        user_id = cookie.id().to_string(),
                        "Received expired user cookie"
                    );
                    Err(ApiError::Unauthorized)?
                }
                if !matches!(cookie.state, LoginState::MfaPending) {
                    warn!(
                        user_id = cookie.id().to_string(),
                        "Received user cookie that is not in `MfaPending` state but {:?}",
                        cookie.state
                    );
                    Err(ApiError::Unauthorized)?
                }
                trace!(
                    user_id = cookie.id().to_string(),
                    "extracted user from session cookie"
                );
                Ok(MfaPending(cookie.id))
            }
            Err(err) => {
                debug!("Invalid session cookie: {err:?}");
                Err(ApiError::Unauthorized)
            }
        }
    }
}

impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiUserRepository: FromRef<S>,
{
    type Rejection = ApiError;

    #[cfg_attr(test, allow(unused_variables))]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        #[cfg(not(test))]
        {
            let Ok(connection) = parts.extract::<ConnectInfo<SocketAddr>>().await else {
                error!("could not determine client IP address");

                return Err(ApiError::BadRequest(
                    "could not determine client IP address".to_string(),
                ));
            };
            trace!("authentication attempt from {}", connection.ip());
        }

        #[cfg(test)]
        {
            if let Some(header) = parts.headers.get("X-Test-Login") {
                trace!("Test log in based on `X-Test-Login` header");
                return match header.to_str().unwrap() {
                    "admin" => Ok(ApiUser::new(Some(Role::Admin), vec![])),
                    token => Ok(ApiUser::new(
                        None,
                        vec![crate::models::OrgRole {
                            role: Role::Admin,
                            org_id: token.parse().unwrap(),
                        }],
                    )),
                };
            } else if let Some(header) = parts.headers.get("X-Test-Login-ID") {
                trace!("Test log in based on `X-Test-Login-ID` header");
                let user_id: ApiUserId = header.to_str().unwrap().parse().unwrap();
                let user = ApiUserRepository::from_ref(state)
                    .find_by_id(&user_id)
                    .await?
                    .ok_or(ApiError::Unauthorized)?;
                return Ok(user);
            }
        }

        let api_state: ApiState = FromRef::from_ref(state);
        let jar = PrivateCookieJar::from_headers(&parts.headers, api_state.config.session_key);

        let session_cookie = jar.get(SESSION_COOKIE_NAME).ok_or(ApiError::Unauthorized)?;

        match serde_json::from_str::<UserCookie>(session_cookie.value()) {
            Ok(user) => {
                if user.expires_at() < &Utc::now() {
                    warn!(
                        user_id = user.id().to_string(),
                        "Received expired user cookie"
                    );
                    Err(ApiError::Unauthorized)?
                }
                if !matches!(user.state, LoginState::LoggedIn) {
                    warn!(
                        user_id = user.id().to_string(),
                        "Received user cookie that is not in `LoggedIn` state but {:?}", user.state
                    );
                    Err(ApiError::Unauthorized)?
                }
                trace!(
                    user_id = user.id().to_string(),
                    "extracted user from session cookie"
                );
                Ok(ApiUserRepository::from_ref(state)
                    .find_by_id(user.id())
                    .await?
                    .ok_or(ApiError::Unauthorized)?)
            }
            Err(err) => {
                debug!("Invalid session cookie: {err:?}");
                Err(ApiError::Unauthorized)
            }
        }
    }
}

impl<S> OptionalFromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiUserRepository: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <ApiUser as FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

impl<S> OptionalFromRequestParts<S> for MfaPending
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiUserRepository: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <MfaPending as FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::TotpCode,
    };
    use axum::body::Body;
    use serde_json::json;
    use sqlx::PgPool;
    use totp_rs::TOTP;

    fn get_session_cookie(response: Response<Body>) -> String {
        let cookies = response.headers().get_all("set-cookie");
        let cookies = cookies.iter().collect::<Vec<_>>();
        assert_eq!(cookies.len(), 1);
        let mut parts = cookies[0].to_str().unwrap().split(';');
        let session = parts
            .find(|s| s.trim().starts_with(&format!("{SESSION_COOKIE_NAME}=")))
            .unwrap();
        session.trim().to_string()
    }

    #[sqlx::test]
    async fn test_password_login(pool: PgPool) {
        let mut server = TestServer::new(pool, None).await;

        // can't get organizations
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // register with password
        let response = server
            .post(
                "/api/register/password",
                serialize_body(json!({
                    "name": "New User",
                    "email": "test-api@new-user",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let session = get_session_cookie(response);

        // now you can get organizations
        server.headers.insert("Cookie", session);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // logout
        let response = server.get("/api/logout").await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let session = get_session_cookie(response);
        assert_eq!(session, format!("{SESSION_COOKIE_NAME}=")); // empty session

        // now you can't get organizations anymore
        server.headers.insert("Cookie", session);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // login with password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-api@new-user",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);

        // now you can get organizations again
        server.headers.insert("Cookie", session);
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // invalid session cookies won't work
        server
            .headers
            .insert("Cookie", "invalid_session".to_string());
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_totp_login(pool: PgPool) {
        let mut server = TestServer::new(pool.clone(), None).await;

        // login with password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);
        server.headers.insert("Cookie", session);

        // Not allowed to set up TOTP for other accounts
        let response = server
            .get("/api/api_user/54432300-128a-46a0-8a83-fe39ce3ce5ef/totp/enroll")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Start 2FA TOTP setup
        let response = server
            .get("/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp/enroll")
            .await
            .unwrap();

        // Verify we get an QR code
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get_all("content-type")
                .iter()
                .collect::<Vec<_>>(),
            vec!["image/png"]
        );

        // get QR code content from the database
        let totp_url = sqlx::query_scalar!(
            r#"
            SELECT url FROM totp WHERE user_id = '820128b1-e08f-404d-ad08-e679a7d6b515' AND state = 'enrolling'
            "#
        ).fetch_one(&pool).await.unwrap();

        // 2FA code is not shown as usable yet
        let response = server
            .get("/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let codes: Vec<TotpCode> = deserialize_body(response.into_body()).await;
        assert_eq!(codes.len(), 0);

        // finish 2FA sign up
        let code = TOTP::from_url(totp_url)
            .unwrap()
            .generate_current()
            .unwrap();
        let response = server
            .post(
                "/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp/enroll",
                serialize_body(json!({
                    "code": code,
                    "description": "test code",
                })),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // verify 2FA code is shown now with the correct description but without a date for "last used"
        let response = server
            .get("/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let codes: Vec<TotpCode> = deserialize_body(response.into_body()).await;
        assert_eq!(codes[0].description, "test code");
        assert!(codes[0].last_used.is_none());

        // logout
        let response = server.get("/api/logout").await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let session = get_session_cookie(response);
        assert_eq!(session, format!("{SESSION_COOKIE_NAME}=")); // empty session
        server.headers.insert("Cookie", session);

        // login with password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);
        server.headers.insert("Cookie", session);

        // requires 2FA before actually logged in
        let whoami = server.get("/api/whoami").await.unwrap();
        assert_eq!(whoami.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(whoami.into_body()).await;
        assert!(matches!(whoami, WhoamiResponse::MfaPending));

        // Cannot yet access any "real" API routes
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = server
            .get("/api/api_user/54432300-128a-46a0-8a83-fe39ce3ce5ef/totp")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // A wrong 2FA code is not accepted
        let response = server
            .post(
                "/api/login/totp",
                serialize_body(serde_json::Value::String("12345".to_string())),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Still can't access API routes
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Finsh login with TOTP
        let response = server
            .post(
                "/api/login/totp",
                serialize_body(serde_json::Value::String(code)),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);
        server.headers.insert("Cookie", session);

        // Can access API routes
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify 2FA code is shown now with the correct description and date for "last used"
        let response = server
            .get("/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let codes: Vec<TotpCode> = deserialize_body(response.into_body()).await;
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].description, "test code");
        assert!(codes[0].last_used.is_some());

        // Can delete TOTP code to disable 2FA
        let response = server
            .delete(format!(
                "/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp/{}",
                codes[0].id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // check that TOTP code deleted
        let response = server
            .get("/api/api_user/820128b1-e08f-404d-ad08-e679a7d6b515/totp")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let codes: Vec<TotpCode> = deserialize_body(response.into_body()).await;
        assert_eq!(codes.len(), 0);

        // logout
        let response = server.get("/api/logout").await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let session = get_session_cookie(response);
        assert_eq!(session, format!("{SESSION_COOKIE_NAME}=")); // empty session
        server.headers.insert("Cookie", session);

        // login with password does not require 2FA anymore
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);
        server.headers.insert("Cookie", session);

        let whoami = server.get("/api/whoami").await.unwrap();
        assert_eq!(whoami.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(whoami.into_body()).await;
        assert!(matches!(whoami, WhoamiResponse::LoggedIn(_)));
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_totp_rate_limit(pool: PgPool) {
        let mut server = TestServer::new(pool.clone(), None).await;

        // get TOTP code secret from the database
        let totp_url = sqlx::query_scalar!(
            r#"
            SELECT url FROM totp WHERE user_id = '672be18f-a89e-4a1d-adaa-45a0b4e2f350' AND state = 'enabled'
            "#
        ).fetch_one(&pool).await.unwrap();
        let totp = TOTP::from_url(totp_url).unwrap();

        // login with password
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp-rate-limit@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session = get_session_cookie(response);
        server.headers.insert("Cookie", session);

        // Four attempts that return "Unauthorized"
        for _ in 0..4 {
            let response = server
                .post(
                    "/api/login/totp",
                    serialize_body(serde_json::Value::String("123456".to_string())),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        // The fifth attempt returns "Too many requests"
        let response = server
            .post(
                "/api/login/totp",
                serialize_body(serde_json::Value::String("123456".to_string())),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        // Also, the correct code won't work right away anymore
        let response = server
            .post(
                "/api/login/totp",
                serialize_body(serde_json::Value::String(totp.generate_current().unwrap())),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        // One minute later, the code should work again
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let response = server
            .post(
                "/api/login/totp",
                serialize_body(serde_json::Value::String(totp.generate_current().unwrap())),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_password_rate_limit(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        // login with the wrong password four times
        for _ in 0..4 {
            let response = server
                .post(
                    "/api/login/password",
                    serialize_body(json!({
                        "email": "test-totp-rate-limit@user-4",
                        "password": "wrong"
                    })),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }

        // the fifth time, the correct password doesn't work either
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp-rate-limit@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // One minute later, the password works again
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let response = server
            .post(
                "/api/login/password",
                serialize_body(json!({
                    "email": "test-totp-rate-limit@user-4",
                    "password": "unsecure"
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_org_access(pool: PgPool) {
        let repo = ApiUserRepository::new(pool);

        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(); // test org 1
        let org_2 = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(); // test org 2

        let user_2 = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // admin of org 2
        let user_3 = "54432300-128a-46a0-8a83-fe39ce3ce5ef".parse().unwrap(); // not in any org
        let user_4 = "c33dbd88-43ed-404b-9367-1659a73c8f3a".parse().unwrap(); // maintainer of org 1
        let user_5 = "703bf1cb-7a3e-4640-83bf-1b07ce18cd2e".parse().unwrap(); // read only in org 1
        let super_admin = "deadbeef-4e43-4a66-bbb9-fbcd4a933a34".parse().unwrap(); // read only in org 1

        let admin = repo.find_by_id(&user_2).await.unwrap().unwrap();
        assert!(admin.has_org_admin_access(&org_1).is_err());
        assert!(admin.has_org_write_access(&org_1).is_err());
        assert!(admin.has_org_read_access(&org_1).is_err());
        assert!(admin.has_org_admin_access(&org_2).is_ok());
        assert!(admin.has_org_write_access(&org_2).is_ok());
        assert!(admin.has_org_read_access(&org_2).is_ok());

        let no_orgs = repo.find_by_id(&user_3).await.unwrap().unwrap();
        assert!(no_orgs.has_org_admin_access(&org_1).is_err());
        assert!(no_orgs.has_org_write_access(&org_1).is_err());
        assert!(no_orgs.has_org_read_access(&org_1).is_err());
        assert!(no_orgs.has_org_admin_access(&org_2).is_err());
        assert!(no_orgs.has_org_write_access(&org_2).is_err());
        assert!(no_orgs.has_org_read_access(&org_2).is_err());

        let maintainer = repo.find_by_id(&user_4).await.unwrap().unwrap();
        assert!(maintainer.has_org_admin_access(&org_1).is_err());
        assert!(maintainer.has_org_write_access(&org_1).is_ok());
        assert!(maintainer.has_org_read_access(&org_1).is_ok());
        assert!(maintainer.has_org_admin_access(&org_2).is_err());
        assert!(maintainer.has_org_write_access(&org_2).is_err());
        assert!(maintainer.has_org_read_access(&org_2).is_err());

        let read_only = repo.find_by_id(&user_5).await.unwrap().unwrap();
        assert!(read_only.has_org_admin_access(&org_1).is_err());
        assert!(read_only.has_org_write_access(&org_1).is_err());
        assert!(read_only.has_org_read_access(&org_1).is_ok());
        assert!(read_only.has_org_admin_access(&org_2).is_err());
        assert!(read_only.has_org_write_access(&org_2).is_err());
        assert!(read_only.has_org_read_access(&org_2).is_err());

        let super_admin = repo.find_by_id(&super_admin).await.unwrap().unwrap();
        assert!(super_admin.has_org_admin_access(&org_1).is_ok());
        assert!(super_admin.has_org_write_access(&org_1).is_ok());
        assert!(super_admin.has_org_read_access(&org_1).is_ok());
        assert!(super_admin.has_org_admin_access(&org_2).is_ok());
        assert!(super_admin.has_org_write_access(&org_2).is_ok());
        assert!(super_admin.has_org_read_access(&org_2).is_ok());
    }
}
