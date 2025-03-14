use crate::api::oauth::GithubOauthService;
use axum::{
    RequestPartsExt,
    extract::{ConnectInfo, FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, request::Parts},
};
use serde::Serialize;
use std::net::SocketAddr;
use tracing::{debug, error, trace};
use uuid::Uuid;

use crate::api::oauth::User;

#[derive(Debug, Clone, Serialize)]
#[allow(unused)]
pub enum Role {
    Admin,
    User(Uuid),
}

#[derive(Debug, Clone)]
pub struct ApiUser {
    pub(super) role: Role,
    pub(super) email: String,
}

impl ApiUser {
    pub fn is_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }

    pub fn is_user(&self) -> bool {
        matches!(self.role, Role::User(_))
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        match self.role {
            Role::User(id) => Some(id),
            _ => None,
        }
    }
}

impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    GithubOauthService: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(connection) = parts.extract::<ConnectInfo<SocketAddr>>().await else {
            error!("could not determine client IP address");

            return Err((
                StatusCode::BAD_REQUEST,
                "could not determine client IP address",
            ));
        };
        let ip = connection.ip();
        trace!("authentication attempt from {ip}");

        if cfg!(test) {
            return if let Some(header) = parts.headers.get("X-Test-Login") {
                trace!("Test log in based on `X-Test-Login` header");
                match header.to_str().unwrap() {
                    "admin" => Ok(ApiUser {
                        role: Role::Admin,
                        email: "admin@remails.com".to_string(),
                    }),
                    token => Ok(ApiUser {
                        role: Role::User(token.parse().unwrap_or_default()),
                        email: "admin@remails.com".to_string(),
                    }),
                }
            } else {
                Err((StatusCode::UNAUTHORIZED, "No valid X-Test-Login header"))
            };
        }

        if let Ok(user) = <User as FromRequestParts<S>>::from_request_parts(parts, state).await {
            trace!("extracted user from session cookie");
            trace!(
                "authenticated request from user {} from ip {ip}",
                user.email
            );
            Ok(ApiUser {
                role: Role::Admin,
                email: user.email,
            })
        } else {
            debug!("No valid session cookie");

            Err((StatusCode::UNAUTHORIZED, "No valid session cookie"))
        }
    }
}

impl<S> OptionalFromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    GithubOauthService: FromRef<S>,
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
