mod api_keys;
mod api_user;
mod domains;
mod invites;
mod message;
mod organization;
mod projects;
mod smtp_credential;

pub(crate) use api_keys::*;
pub(crate) use api_user::*;
pub(crate) use domains::*;
pub(crate) use invites::*;
pub(crate) use message::*;
pub(crate) use organization::*;
pub(crate) use projects::*;
pub(crate) use smtp_credential::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(sqlx::Error),
    #[error("foreign key violation")]
    ForeignKeyViolation,
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Email(#[from] email_address::Error),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Internal(String),
    #[error("AWS Cryptographic error {0}")]
    AwsCrypto(#[from] aws_lc_rs::error::Unspecified),
    #[error("AWS Cryptographic key rejected {0}")]
    WrongCryptKey(#[from] aws_lc_rs::error::KeyRejected),
    #[error("Email Authentication error{0}")]
    MailAuth(#[from] mail_auth::Error),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("conflict")]
    Conflict,
    #[error("invalid utf8")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("totp error")]
    Totp(#[from] totp_rs::TotpUrlError),
    #[error("too many requests, try again later")]
    TooManyRequests,
    #[error("organization has been blocked")]
    OrgBlocked,
}

impl From<sqlx::Error> for Error {
    fn from(sql: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &sql {
            if db_err.is_unique_violation() {
                return Error::Conflict;
            }
            if db_err.is_foreign_key_violation() {
                return Error::ForeignKeyViolation;
            }
        }
        if matches!(sql, sqlx::Error::RowNotFound) {
            return Error::NotFound("not found");
        }
        Error::Database(sql)
    }
}
