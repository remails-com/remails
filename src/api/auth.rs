use crate::{
    api::{ApiState, error::ApiError, validation::ValidatedJson, whoami::WhoamiResponse},
    models::{
        ApiKey, ApiKeyRepository, ApiUser, ApiUserId, ApiUserRepository, NewApiUser,
        OrganizationId, Password, Role, TotpCode,
    },
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
use base64ct::Encoding;
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use email_address::EmailAddress;
use garde::Validate;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
#[cfg(not(test))]
use std::net::SocketAddr;
#[cfg(not(test))]
use tracing::error;
use tracing::{debug, trace, warn};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Cookies#cookie_prefixes
pub static SESSION_COOKIE_NAME: &str = "__Host-SESSION";

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(password_login))
        .routes(routes!(totp_login))
        .routes(routes!(password_register))
        .routes(routes!(logout))
}

/// Objects implementing `Authenticated` may be allowed to use the Remails API if they have the
/// right level of permissions for the organization.
/// This is currently implemented by `ApiUser`s, who are logged in through the Remails web
/// interface, and `ApiKey`s, which can be created by `ApiUser`s.
///
/// Note that certain parts of the Remails API may only be accessible for real `ApiUser`s, such
/// as the API end-points for creating `ApiKey`s.
pub trait Authenticated: Send + Sync {
    /// Check if user has a certain access level within an organization
    fn is_at_least(&self, org_id: &OrganizationId, role: Role) -> bool;

    fn has_org_read_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        self.is_at_least(org_id, Role::ReadOnly)
            .then_some(())
            .ok_or(ApiError::forbidden())
    }

    fn has_org_write_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        self.is_at_least(org_id, Role::Maintainer)
            .then_some(())
            .ok_or(ApiError::forbidden())
    }

    fn has_org_admin_access(&self, org_id: &OrganizationId) -> Result<(), ApiError> {
        self.is_at_least(org_id, Role::Admin)
            .then_some(())
            .ok_or(ApiError::forbidden())
    }

    /// Get a list of the UUIDs of all organizations that are viewable by this user,
    /// or None if user is allowed to view all organizations from everyone (for super admins)
    fn viewable_organizations_filter(&self) -> Option<Vec<uuid::Uuid>>;

    /// Get a string with an ID for logging, also include some information about the type of
    /// Authenticated object that is used
    fn log_id(&self) -> String;
}

impl ApiUser {
    /// Check if user is super admin (has access to all organizations)
    pub fn is_super_admin(&self) -> bool {
        self.global_role
            .as_ref()
            .is_some_and(|role| *role == Role::Admin)
    }
}

impl Authenticated for ApiUser {
    fn is_at_least(&self, org_id: &OrganizationId, role: Role) -> bool {
        self.is_super_admin()
            || self
                .org_roles
                .iter()
                .any(|org_role| org_role.org_id == *org_id && org_role.role.is_at_least(role))
    }

    fn viewable_organizations_filter(&self) -> Option<Vec<uuid::Uuid>> {
        if self.is_super_admin() {
            None // show all organizations
        } else {
            Some(
                self.org_roles
                    .iter()
                    .map(|org_role| *org_role.org_id)
                    .collect(),
            )
        }
    }

    fn log_id(&self) -> String {
        format!("ApiUser {}", self.id())
    }
}

impl Authenticated for ApiKey {
    fn is_at_least(&self, org_id: &OrganizationId, role: Role) -> bool {
        org_id == self.organization_id() && self.role().is_at_least(role)
    }

    fn viewable_organizations_filter(&self) -> Option<Vec<uuid::Uuid>> {
        // API keys can only see one organization
        Some(vec![**self.organization_id()])
    }

    fn log_id(&self) -> String {
        format!("ApiKey {}", self.id())
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

    fn from_api_user(user: &ApiUser, login_state: LoginState) -> Self {
        Self {
            id: *user.id(),
            state: login_state,
            expires_at: Utc::now() + Duration::days(7),
        }
    }
}

#[derive(Deserialize, ToSchema, Validate)]
pub(super) struct PasswordLogin {
    #[garde(skip)]
    email: EmailAddress,
    #[garde(dive)]
    #[schema(min_length = 10, max_length = 256)]
    password: Password,
}

/// Password login
///
/// Returns an authentication cookie.
/// If the user configured 2FA, you have to call the corresponding path hereafter,
/// currently only TOTP (`/login/totp`) is supported
#[utoipa::path(post, path = "/login/password",
    tags = ["internal", "Auth"],
    security(()),
    request_body = PasswordLogin,
    responses(
        (status = 200, description = "Successfully logged in", body = WhoamiResponse,
            headers(
                ("set-cookie", description = "sets the authentication cookie")
        )),
        ApiError,
))]
pub(super) async fn password_login(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    ValidatedJson(login_attempt): ValidatedJson<PasswordLogin>,
) -> Result<Response, ApiError> {
    repo.check_password(&login_attempt.email, login_attempt.password)
        .await?;
    let user = repo
        .find_by_email(&login_attempt.email)
        .await?
        .ok_or(ApiError::not_found())?;

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

/// TOTP login
///
/// Second factor authentication with Time-Based One-time Password (TOTP).
/// Returns an updated authentication cookie
#[utoipa::path(post, path = "/login/totp",
    tags = ["internal", "Auth"],
    request_body = TotpCode,
    security(("cookieAuth" = [])),
    responses(
        (status = 200, description = "Successfully logged in", body = WhoamiResponse,
            headers(
                ("set-cookie", description = "sets the authentication cookie")
        )),
        ApiError,
))]
pub(super) async fn totp_login(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    user: MfaPending,
    ValidatedJson(totp_code): ValidatedJson<TotpCode>,
) -> Result<Response, ApiError> {
    if !repo.check_totp_code(&user.id(), totp_code.as_ref()).await? {
        return Ok((StatusCode::UNAUTHORIZED, Json(WhoamiResponse::MfaPending)).into_response());
    }

    let user = repo
        .find_by_id(&user.id())
        .await?
        .ok_or(ApiError::unauthorized())?;
    cookie_storage = login(&user, LoginState::LoggedIn, cookie_storage)?;

    Ok((
        StatusCode::OK,
        cookie_storage,
        Json(WhoamiResponse::logged_in(user)),
    )
        .into_response())
}

#[derive(Deserialize, Validate, ToSchema)]
pub(super) struct PasswordRegister {
    #[garde(length(min = 1, max = 256))]
    #[schema(min_length = 1, max_length = 256)]
    name: String,
    #[garde(skip)]
    email: EmailAddress,
    #[garde(dive)]
    #[schema(min_length = 10, max_length = 256)]
    password: Password,
}

/// Register with password
///
/// Creates a new ApiUser and returns the corresponding authentication cookie.
#[utoipa::path(post, path = "/register/password",
    tags = ["internal", "Auth"],
    security(()),
    request_body = PasswordRegister,
    responses(
        (status = 201, description = "Successfully created API user", body = WhoamiResponse,
            headers(
                ("set-cookie", description = "sets the authentication cookie")
        )),
        ApiError,
))]
pub(super) async fn password_register(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    ValidatedJson(register_attempt): ValidatedJson<PasswordRegister>,
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

/// Logout
#[utoipa::path(get, path = "/logout",
    tags = ["internal", "Auth"],
    security(()),
    responses(
        (status = 303, description = "Successfully logged out",
            headers(
                ("set-cookie", description = "removes the authentication cookie")
        )),
        ApiError,
))]
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
        let jar =
            PrivateCookieJar::from_headers(&parts.headers, api_state.config.session_key.clone());

        let session_cookie = jar
            .get(SESSION_COOKIE_NAME)
            .ok_or(ApiError::unauthorized())?;

        match serde_json::from_str::<UserCookie>(session_cookie.value()) {
            Ok(cookie) => {
                if cookie.expires_at() < &Utc::now() {
                    warn!(
                        user_id = cookie.id().to_string(),
                        "Received expired user cookie"
                    );
                    Err(ApiError::unauthorized())?
                }
                if !matches!(cookie.state, LoginState::MfaPending) {
                    Err(ApiError::unauthorized())?
                }
                trace!(
                    user_id = cookie.id().to_string(),
                    "extracted user from session cookie"
                );
                Ok(MfaPending(cookie.id))
            }
            Err(err) => {
                debug!("Invalid session cookie: {err:?}");
                Err(ApiError::unauthorized())
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

                return Err(ApiError::bad_request(
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
                    .ok_or(ApiError::unauthorized())?;
                return Ok(user);
            }
        }

        let api_state: ApiState = FromRef::from_ref(state);
        let jar =
            PrivateCookieJar::from_headers(&parts.headers, api_state.config.session_key.clone());

        let session_cookie = jar
            .get(SESSION_COOKIE_NAME)
            .ok_or(ApiError::unauthorized())?;

        match serde_json::from_str::<UserCookie>(session_cookie.value()) {
            Ok(user) => {
                if user.expires_at() < &Utc::now() {
                    warn!(
                        user_id = user.id().to_string(),
                        "Received expired user cookie"
                    );
                    Err(ApiError::unauthorized())?
                }
                if !matches!(user.state, LoginState::LoggedIn) {
                    warn!(
                        user_id = user.id().to_string(),
                        "Received user cookie that is not in `LoggedIn` state but {:?}", user.state
                    );
                    Err(ApiError::unauthorized())?
                }
                trace!(
                    user_id = user.id().to_string(),
                    "extracted user from session cookie"
                );
                Ok(ApiUserRepository::from_ref(state)
                    .find_by_id(user.id())
                    .await?
                    .ok_or(ApiError::unauthorized())?)
            }
            Err(err) => {
                debug!("Invalid session cookie: {err:?}");
                Err(ApiError::unauthorized())
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

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiKeyRepository: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // check HTTP Basic Auth header for API keys
        let Some(header) = parts.headers.get("Authorization") else {
            return Err(ApiError::unauthorized());
        };

        trace!("Logging in based on `Authorization` header");
        let header = header.to_str().map_err(|_| ApiError::unauthorized())?;

        let (auth_type, base64_credentials) =
            header.split_once(' ').ok_or(ApiError::unauthorized())?;

        if auth_type.to_lowercase() != "basic" {
            return Err(ApiError::unauthorized());
        }

        let credentials = base64ct::Base64::decode_vec(base64_credentials)
            .map_err(|_| ApiError::unauthorized())?;
        let credentials = String::from_utf8(credentials).map_err(|_| ApiError::unauthorized())?;

        let (api_key_id, password) = credentials
            .split_once(':')
            .ok_or(ApiError::unauthorized())?;

        let api_key_id = api_key_id.parse().map_err(|_| ApiError::unauthorized())?;
        let api_key = ApiKeyRepository::from_ref(state)
            .get(api_key_id)
            .await
            .map_err(|_| ApiError::unauthorized())?;

        if api_key.verify_password(&Password::new(password.to_owned())) {
            Ok(api_key)
        } else {
            Err(ApiError::unauthorized())
        }
    }
}

impl<S> FromRequestParts<S> for Box<dyn Authenticated>
where
    S: Send + Sync,
    ApiState: FromRef<S>,
    ApiUserRepository: FromRef<S>,
    ApiKeyRepository: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // first check if they are using a valid API key
        if let Ok(api_key) = <ApiKey as FromRequestParts<S>>::from_request_parts(parts, state).await
        {
            return Ok(Box::new(api_key));
        }

        // otherwise, check if they are logged in as an ApiUser
        let api_user = <ApiUser as FromRequestParts<S>>::from_request_parts(parts, state).await?;
        Ok(Box::new(api_user))
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
        models::TotpCodeDetails,
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
                    "password": "unsecure123"
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
                    "password": "unsecure123"
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
                    "password": "unsecure123"
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
        let codes: Vec<TotpCodeDetails> = deserialize_body(response.into_body()).await;
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
        let codes: Vec<TotpCodeDetails> = deserialize_body(response.into_body()).await;
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
                    "password": "unsecure123"
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Still can't access API routes
        let response = server.get("/api/organizations").await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Finish login with TOTP
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
        let codes: Vec<TotpCodeDetails> = deserialize_body(response.into_body()).await;
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
        let codes: Vec<TotpCodeDetails> = deserialize_body(response.into_body()).await;
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
                    "password": "unsecure123"
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
                    "password": "unsecure123"
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
                        "password": "wrongwrong"
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
                    "password": "unsecure123"
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
                    "password": "unsecure123"
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
