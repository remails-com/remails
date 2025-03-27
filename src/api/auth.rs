use crate::{
    api::{ApiState, error::ApiError},
    models::{ApiUser, ApiUserId, ApiUserRepository, ApiUserRole, OrganizationId},
};
use axum::{
    RequestPartsExt,
    extract::{ConnectInfo, FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, IntoResponseParts, Redirect, ResponseParts},
};
use axum_extra::extract::PrivateCookieJar;
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr};
#[cfg(not(test))]
use tracing::debug;
use tracing::{error, trace, warn};

static SESSION_COOKIE_NAME: &str = "SESSION";

impl ApiUser {
    pub fn is_super_admin(&self) -> bool {
        self.roles()
            .iter()
            .any(|r| matches!(r, ApiUserRole::SuperAdmin))
    }

    pub fn org_admin(&self) -> Vec<OrganizationId> {
        self.roles().iter().fold(Vec::new(), |mut acc, role| {
            if let ApiUserRole::OrganizationAdmin(org) = role {
                acc.push(*org);
            };
            acc
        })
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

impl UserCookie {
    pub fn id(&self) -> &ApiUserId {
        &self.id
    }
    pub fn expires_at(&self) -> &DateTime<Utc> {
        &self.expires_at
    }
}

impl From<ApiUser> for UserCookie {
    fn from(user: ApiUser) -> Self {
        Self {
            id: *user.id(),
            expires_at: Utc::now() + Duration::days(7),
        }
    }
}

pub(super) fn login(
    user: ApiUser,
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

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(connection) = parts.extract::<ConnectInfo<SocketAddr>>().await else {
            error!("could not determine client IP address");

            return Err(ApiError::BadRequest(
                "could not determine client IP address".to_string(),
            ));
        };
        let ip = connection.ip();
        trace!("authentication attempt from {ip}");

        #[cfg(test)]
        {
            if let Some(header) = parts.headers.get("X-Test-Login") {
                trace!("Test log in based on `X-Test-Login` header");
                match header.to_str().unwrap() {
                    "admin" => Ok(ApiUser::new(vec![ApiUserRole::SuperAdmin])),
                    token => Ok(ApiUser::new(vec![ApiUserRole::OrganizationAdmin(
                        token.parse().unwrap(),
                    )])),
                }
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
                        .find_by_id(*user.id())
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
