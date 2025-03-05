use crate::api::oauth::{GithubOauthService, User};
use axum::extract::FromRef;
use axum::{
    RequestPartsExt,
    extract::{ConnectInfo, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use std::net::{IpAddr, SocketAddr};
use tracing::{debug, error, trace};
use uuid::Uuid;

#[derive(Debug, Clone)]
enum Role {
    Admin,
    User(Uuid),
}

#[derive(Debug, Clone)]
pub struct ApiUser {
    ip: IpAddr,
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

    #[cfg(test)]
    fn new_admin(ip: IpAddr) -> Self {
        Self {
            ip,
            role: Role::Admin,
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
        trace!("extracting user from session cookie");

        #[cfg(test)]
        if let Some(header) = parts.headers.get("X-Test-Login") {
            return match header.to_str().unwrap() {
                "admin" => Ok(ApiUser {
                    ip,
                    role: Role::Admin,
                }),
                token => Ok(ApiUser {
                    ip,
                    role: Role::User(token.parse().unwrap_or_default()),
                }),
            };
        } else {
            Err((StatusCode::UNAUTHORIZED, "No valid X-Test-Login header"))
        }

        #[cfg(not(test))]
        if let Ok(user) = User::from_request_parts(parts, state).await {
            trace!(
                "authenticated request from user {} from ip {ip}",
                user.email
            );
            Ok(ApiUser {
                ip,
                role: Role::Admin,
            })
        } else {
            debug!("No valid session cookie");

            Err((StatusCode::UNAUTHORIZED, "No valid session cookie"))
        }
    }
}
