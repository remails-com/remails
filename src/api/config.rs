use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::SmtpConfig;

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub server_name: String,
    pub address: String,
    pub port: u16,
}

impl From<SmtpConfig> for ConfigResponse {
    fn from(config: SmtpConfig) -> Self {
        ConfigResponse {
            server_name: config.server_name,
            address: config.listen_addr.ip().to_string(),
            port: config.listen_addr.port(),
        }
    }
}

pub async fn config(State(smtp_config): State<SmtpConfig>) -> Response {
    Json(ConfigResponse::from(smtp_config)).into_response()
}
