use crate::api::error::ApiError;
use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use garde::Validate;
use serde::de::DeserializeOwned;

pub(crate) struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    <T as Validate>::Context: Default,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}
