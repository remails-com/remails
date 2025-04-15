mod api_user;
mod message;
mod organization;
mod smtp_credential;

pub(crate) use api_user::*;
pub(crate) use message::*;
pub(crate) use organization::*;
use serde::Serialize;
pub(crate) use smtp_credential::*;
use sqlx_paginated::PaginatedResponse;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Email(#[from] email_address::Error),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("conflict")]
    Conflict,
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
