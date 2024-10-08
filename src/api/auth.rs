use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    RequestPartsExt,
};
use std::net::{IpAddr, SocketAddr};
use tracing::{debug, error, trace};
use uuid::Uuid;

#[allow(unused)]
#[derive(Debug, Clone)]
enum Role {
    Admin,
    User(Uuid),
}

#[allow(unused)]
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
}

#[async_trait]
impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        trace!("extracting `Authorization` header");

        if let Some(authorization) = parts.headers.get(AUTHORIZATION).cloned() {
            trace!("authorization header: {authorization:?}");

            let (prefix, key) = authorization
                .to_str()
                .unwrap_or_default()
                .split_once(' ')
                .unwrap_or(("", ""));

            if prefix != "Bearer" {
                error!("invalid `Authorization` header prefix");

                return Err((
                    StatusCode::BAD_REQUEST,
                    "invalid `Authorization` header prefix",
                ));
            }

            let Ok(connection) = parts.extract::<ConnectInfo<SocketAddr>>().await else {
                error!("could not determine client IP address");

                return Err((
                    StatusCode::BAD_REQUEST,
                    "could not determine client IP address",
                ));
            };

            let ip = connection.ip();

            debug!("authentication attempt from {ip} with key '{key}'");

            #[cfg(test)]
            match key {
                "admin" => Ok(ApiUser {
                    ip,
                    role: Role::Admin,
                }),
                token => Ok(ApiUser {
                    ip,
                    role: Role::User(token.parse().unwrap_or_default()),
                }),
            }

            #[cfg(not(test))]
            Err((StatusCode::UNAUTHORIZED, "invalid token"))
        } else {
            error!("`Authorization` header is missing");

            Err((StatusCode::BAD_REQUEST, "`User-Agent` header is missing"))
        }
    }
}
