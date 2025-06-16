use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::api::RemailsConfig;

pub async fn config(State(config): State<RemailsConfig>) -> Response {
    Json(config).into_response()
}
