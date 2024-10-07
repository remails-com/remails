use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    RequestPartsExt,
};
use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;

const TEST_ADMIN_TOKEN: &str = "admin";
const TEST_COMPANY_TOKEN: &str = "user";

#[allow(unused)]
#[derive(Debug, Clone)]
enum Role {
    Admin,
    Company(Uuid),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ApiUser {
    ip: IpAddr,
    role: Role,
}

#[async_trait]
impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        if let Some(authorization) = parts.headers.get(AUTHORIZATION).cloned() {
            let (prefix, key) = authorization
                .to_str()
                .unwrap_or_default()
                .split_once(' ')
                .unwrap_or(("", ""));

            if prefix != "Bearer" {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "invalid `Authorization` header prefix",
                ));
            }

            let Ok(connection) = parts.extract::<ConnectInfo<SocketAddr>>().await else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "could not determine client IP address",
                ));
            };

            let ip = connection.ip();

            match key {
                TEST_ADMIN_TOKEN => {
                    return Ok(ApiUser {
                        ip,
                        role: Role::Admin,
                    });
                }
                TEST_COMPANY_TOKEN => {
                    return Ok(ApiUser {
                        ip,
                        role: Role::Company(Uuid::new_v4()),
                    });
                }
                _ => {
                    return Err((StatusCode::UNAUTHORIZED, "invalid token"));
                }
            }
        } else {
            Err((StatusCode::BAD_REQUEST, "`User-Agent` header is missing"))
        }
    }
}
