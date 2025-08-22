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
use tracing::{debug, error};
use tracing::{trace, warn};

static SESSION_COOKIE_NAME: &str = "SESSION";

impl ApiUser {
    pub fn is_super_admin(&self) -> bool {
        self.global_role
            .as_ref()
            .is_some_and(|role| *role == Role::Admin)
    }

    pub fn is_org_admin(&self, org_id: &OrganizationId) -> bool {
        self.org_roles
            .iter()
            .any(|org_role| org_role.org_id == *org_id && org_role.role == Role::Admin)
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
pub struct UserCookie {
    id: ApiUserId,
    expires_at: DateTime<Utc>,
}

#[cfg(not(test))]
impl UserCookie {
    pub fn id(&self) -> &ApiUserId {
        &self.id
    }
    pub fn expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }
}

impl From<&ApiUser> for UserCookie {
    fn from(user: &ApiUser) -> Self {
        Self {
            id: *user.id(),
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
    cookie_storage = login(&user, cookie_storage)?;
    let whoami = WhoamiResponse::from(user);
    Ok((StatusCode::OK, cookie_storage, Json(whoami)).into_response())
}

#[derive(Deserialize)]
pub(super) struct PasswordRegister {
    name: String,
    email: EmailAddress,
    password: Password,
    terms: bool,
}

pub(super) async fn password_register(
    State(repo): State<ApiUserRepository>,
    mut cookie_storage: SecureCookieStorage,
    Json(register_attempt): Json<PasswordRegister>,
) -> Result<Response, ApiError> {
    if !register_attempt.terms {
        return Err(ApiError::BadRequest(
            "You must accept the terms and conditions".to_string(),
        ));
    }
    let new = NewApiUser {
        email: register_attempt.email,
        name: register_attempt.name.trim().to_string(),
        password: Some(register_attempt.password),
        global_role: None,
        org_roles: vec![],
        github_user_id: None,
    };

    let user = repo.create(new).await?;

    cookie_storage = login(&user, cookie_storage)?;
    let whoami = WhoamiResponse::from(user);
    Ok((StatusCode::CREATED, cookie_storage, Json(whoami)).into_response())
}

pub(super) fn login(
    user: &ApiUser,
    cookie_storage: SecureCookieStorage,
) -> Result<SecureCookieStorage, serde_json::Error> {
    // Serialize the user data as a string
    let cookie: UserCookie = user.into();
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
            let ip = connection.ip();
            trace!("authentication attempt from {ip}");
        }

        #[cfg(test)]
        {
            if let Some(header) = parts.headers.get("X-Test-Login") {
                trace!("Test log in based on `X-Test-Login` header");
                match header.to_str().unwrap() {
                    "admin" => Ok(ApiUser::new(Some(Role::Admin), vec![])),
                    token => Ok(ApiUser::new(
                        None,
                        vec![crate::models::OrgRole {
                            role: Role::Admin,
                            org_id: token.parse().unwrap(),
                        }],
                    )),
                }
            } else if let Some(header) = parts.headers.get("X-Test-Login-ID") {
                trace!("Test log in based on `X-Test-Login-ID` header");
                let user_id: ApiUserId = header.to_str().unwrap().parse().unwrap();
                let user = ApiUserRepository::from_ref(state)
                    .find_by_id(&user_id)
                    .await?
                    .ok_or(ApiError::Unauthorized)?;
                Ok(user)
            } else {
                warn!("No valid X-Test-Login header");
                Err(ApiError::Unauthorized)
            }
        }

        #[cfg(not(test))]
        {
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
