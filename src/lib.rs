use api::ApiServer;
use handler::Handler;
use models::{Message, SmtpCredentialRepository};
use smtp::smtp_server::SmtpServer;
use sqlx::PgPool;
use std::net::SocketAddrV4;
use tokio::{signal, sync::mpsc};
use tokio_util::sync::CancellationToken;

mod api;
mod handler;
mod smtp;

mod models;
#[cfg(test)]
mod test;

pub async fn run_mta(pool: PgPool, smtp_socket: SocketAddrV4, shutdown: CancellationToken) {
    let user_repository = SmtpCredentialRepository::new(pool.clone());

    let (queue_sender, queue_receiver) = mpsc::channel::<Message>(100);

    let smtp_server = SmtpServer::new(
        smtp_socket,
        "cert.pem".into(),
        "key.pem".into(),
        user_repository,
        queue_sender,
        shutdown.clone(),
    );

    let message_handler = Handler::new(pool.clone(), shutdown);

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
