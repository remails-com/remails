use crate::api::error::AppError;
use axum::{
    Json,
    extract::{
        FromRequest, FromRequestParts, Query, Request,
        rejection::{BytesRejection, FailedToBufferBody, JsonRejection, QueryRejection},
    },
};
use garde::Validate;
use http::request::Parts;
use serde::de::DeserializeOwned;

pub(crate) struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    <T as Validate>::Context: Default,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|err| {
            if matches!(
                err,
                JsonRejection::BytesRejection(BytesRejection::FailedToBufferBody(
                    FailedToBufferBody::LengthLimitError(_)
                ))
            ) {
                AppError::PayloadTooLarge
            } else {
                err.into()
            }
        })?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

pub(crate) struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    <T as Validate>::Context: Default,
    S: Send + Sync,
    Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
{
    type Rejection = AppError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(req, state).await?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}
