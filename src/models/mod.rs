/// Define a transparent UUID newtype
macro_rules! id {
    ($(#[$attr:meta])* $name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            serde::Deserialize,
            serde::Serialize,
            PartialEq,
            PartialOrd,
            derive_more::From,
            derive_more::Display,
            derive_more::Deref,
            derive_more::FromStr,
            sqlx::Type,
            utoipa::ToSchema,
        )]
        #[sqlx(transparent)]
        $(#[$attr])*
        pub struct $name(uuid::Uuid);
    };
}

mod api_keys;
mod api_user;
mod audit_log;
mod domains;
mod error;
mod invites;
mod labels;
mod message;
mod organization;
mod projects;
mod runtime_config;
mod smtp_credential;
mod statistics;
mod suppressed;

pub(crate) use api_keys::*;
pub(crate) use api_user::*;
pub(crate) use audit_log::*;
pub(crate) use domains::*;
pub(crate) use error::Error;
pub(crate) use invites::*;
pub(crate) use labels::*;
pub(crate) use message::*;
pub(crate) use organization::*;
pub(crate) use projects::*;
pub(crate) use runtime_config::*;
pub(crate) use smtp_credential::*;
pub(crate) use statistics::*;
pub(crate) use suppressed::*;
