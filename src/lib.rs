use api::ApiServer;
use derive_more::FromStr;
use handler::Handler;
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddrV4, sync::Arc};
use tokio::{signal, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod api;
mod dkim;
pub mod handler;
pub mod periodically;
mod smtp;
use crate::bus::client::BusClient;
pub use crate::handler::HandlerConfig;
pub use smtp::{SmtpConfig, server::SmtpServer};
pub mod bus;

#[cfg(feature = "load-fixtures")]
pub use fixtures::load_fixtures;

mod models;
#[cfg(test)]
mod test;

#[cfg(feature = "load-fixtures")]
mod fixtures;

mod kubernetes;
mod moneybird;

pub use kubernetes::Kubernetes;
pub use moneybird::*;

#[derive(Debug, Default, Clone, Copy, FromStr, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub enum Environment {
    Staging,
    Production,
    #[default]
    Development,
}

impl Environment {
    pub fn from_env() -> Self {
        env::var("ENVIRONMENT")
            .map(|s| s.parse())
            .inspect_err(|_| warn!("Did not find ENVIRONMENT env var, defaulting to development"))
            .unwrap_or(Ok(Environment::Development))
            .expect(
                "Invalid ENVIRONMENT env var, must be one of: development, production, or staging",
            )
    }
}

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=trace,tower_http=debug,axum=trace,info",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

pub async fn run_mta(
    pool: PgPool,
    smtp_config: SmtpConfig,
    handler_config: HandlerConfig,
    bus_client: BusClient,
    shutdown: CancellationToken,
) {
    let smtp_config = Arc::new(smtp_config);
    let handler_config = Arc::new(handler_config);

    let smtp_server = SmtpServer::new(
        pool.clone(),
        smtp_config,
        bus_client.clone(),
        shutdown.clone(),
    );

    let message_handler = Handler::new(pool, handler_config, bus_client, shutdown).await;

    smtp_server.spawn();
    message_handler.spawn();
}

pub async fn run_api_server(
    pool: PgPool,
    http_socket: SocketAddrV4,
    shutdown: CancellationToken,
    with_frontend: bool,
) -> JoinHandle<()> {
    let bus = BusClient::new_from_env_var().expect("Could not connect to message bus");
    let api_server = ApiServer::new(
        http_socket.into(),
        pool.clone(),
        shutdown,
        with_frontend,
        bus,
    )
    .await;

    api_server.spawn()
}

pub async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = token.cancelled() => {},
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
