mod api_user;
mod domains;
mod message;
mod organization;
mod projects;
mod smtp_credential;
mod streams;

pub(crate) use api_user::*;
pub(crate) use domains::*;
pub(crate) use message::*;
pub(crate) use organization::*;
pub(crate) use projects::*;
use serde::Serialize;
pub(crate) use smtp_credential::*;
use sqlx_paginated::PaginatedResponse;
pub(crate) use streams::*;
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
    MailAuth(#[from] mail_send::mail_auth::Error),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("conflict")]
    Conflict,
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

#[derive(Serialize, Debug)]
pub struct Paginated<T> {
    pub records: Vec<T>,
    pub total: Option<i64>,
    pub total_pages: Option<i64>,
}

impl<T> From<PaginatedResponse<T>> for Paginated<T> {
    fn from(p: PaginatedResponse<T>) -> Self {
        Self {
            records: p.records,
            total: p.total,
            total_pages: p.total_pages,
        }
    }
}
