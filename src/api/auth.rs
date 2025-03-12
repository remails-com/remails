use crate::api::oauth::GithubOauthService;
#[cfg(not(test))]
use crate::api::oauth::User;
use axum::{
    RequestPartsExt,
    extract::{ConnectInfo, FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use std::net::SocketAddr;
#[cfg(not(test))]
use tracing::debug;
use tracing::{error, trace};
use uuid::Uuid;

#[derive(Debug, Clone)]
#[allow(unused)]
enum Role {
    Admin,
    User(Uuid),
}

#[derive(Debug, Clone)]
pub struct ApiUser {
    role: Role,
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

    #[cfg_attr(test, allow(unused))]
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
        trace!("extracting user from session cookie");

        #[cfg(test)]
        if let Some(header) = parts.headers.get("X-Test-Login") {
            match header.to_str().unwrap() {
                "admin" => Ok(ApiUser { role: Role::Admin }),
                token => Ok(ApiUser {
                    role: Role::User(token.parse().unwrap_or_default()),
                }),
            }
        } else {
            Err((StatusCode::UNAUTHORIZED, "No valid X-Test-Login header"))
        }

        #[cfg(not(test))]
        if let Ok(user) = User::from_request_parts(parts, state).await {
            trace!(
                "authenticated request from user {} from ip {ip}",
                user.email
            );
            Ok(ApiUser { role: Role::Admin })
        } else {
            debug!("No valid session cookie");

            Err((StatusCode::UNAUTHORIZED, "No valid session cookie"))
        }
    }
}
