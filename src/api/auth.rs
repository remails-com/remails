#[cfg(not(test))]
use crate::models::UserCookie;
use crate::{
    api::{ApiState, error::ApiError},
    models::{ApiUser, ApiUserRepository, ApiUserRole, OrganizationId},
};
use axum::{
    RequestPartsExt, debug_handler,
    extract::{ConnectInfo, FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, request::Parts},
    response::IntoResponse,
};
use axum_extra::extract::PrivateCookieJar;
#[cfg(not(test))]
use chrono::Utc;
use cookie::SameSite;
use std::{convert::Infallible, net::SocketAddr};
#[cfg(not(test))]
use tracing::debug;
use tracing::{error, trace, warn};

pub(crate) static SESSION_COOKIE_NAME: &str = "SESSION";

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

pub(crate) struct CookieStorage {
    pub(crate) jar: PrivateCookieJar,
}

impl FromRequestParts<ApiState> for CookieStorage {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        // let state = <ApiState as FromRequestParts<S>>::from_request_parts(parts, state).await.ok().unwrap();
        let jar = PrivateCookieJar::from_headers(&parts.headers, state.config.session_key.clone());

        Ok(Self { jar })
    }
}

/// Handles the logout request.
/// Removes the session cookie from the cookie jar and returns a simple message indicating successful logout.
///
/// # Parameters
///
/// - `jar`: The private cookie jar containing the session cookie.
///
/// # Returns
///
/// Returns a tuple containing the updated cookie jar and a simple logout message.
#[debug_handler(state= ApiState)]
pub(super) async fn logout(storage: CookieStorage) -> impl IntoResponse {
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

    // Return the updated cookie jar and a logout message
    (jar, "You are now logged out 👋")
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
