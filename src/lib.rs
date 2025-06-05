use crate::models::NewMessage;
use api::ApiServer;
use handler::Handler;
use models::SmtpCredentialRepository;
use smtp::server::SmtpServer;
use sqlx::PgPool;
use std::{net::SocketAddrV4, sync::Arc};
use tokio::{signal, sync::mpsc};
use tokio_util::sync::CancellationToken;

pub mod api;
mod dkim;
pub mod handler;
mod smtp;
pub use crate::handler::HandlerConfig;
pub use smtp::SmtpConfig;

#[cfg(feature = "load-fixtures")]
pub use fixtures::load_fixtures;

mod models;
#[cfg(test)]
mod test;

#[cfg(feature = "load-fixtures")]
mod fixtures;

pub async fn run_mta(
    pool: PgPool,
    smtp_config: SmtpConfig,
    handler_config: HandlerConfig,
    shutdown: CancellationToken,
) {
    let smtp_config = Arc::new(smtp_config);
    let handler_config = Arc::new(handler_config);
    let user_repository = SmtpCredentialRepository::new(pool.clone());

    let (queue_sender, queue_receiver) = mpsc::channel::<NewMessage>(100);

    let smtp_server = SmtpServer::new(smtp_config, user_repository, queue_sender, shutdown.clone());

    let message_handler = Handler::new(pool.clone(), handler_config, shutdown.clone());

    smtp_server.spawn();
    message_handler.spawn(queue_receiver);
}

pub async fn run_api_server(
    pool: PgPool,
    http_socket: SocketAddrV4,
    shutdown: CancellationToken,
    with_frontend: bool,
) {
    let mut api_server = ApiServer::new(http_socket.into(), pool.clone(), shutdown).await;

    if with_frontend {
        api_server = api_server.serve_frontend().await;
    }

    api_server.spawn();
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
