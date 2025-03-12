use api::ApiServer;
use handler::Handler;
use message::Message;
use smtp::smtp_server::SmtpServer;
use sqlx::PgPool;
use std::net::SocketAddrV4;
use tokio::{signal, sync::mpsc};
use tokio_util::sync::CancellationToken;
use user::UserRepository;

mod api;
mod handler;
mod message;
mod smtp;
mod user;

#[cfg(test)]
mod test;

pub async fn run_mta(pool: PgPool, smtp_socket: SocketAddrV4) -> CancellationToken {
    let user_repository = UserRepository::new(pool.clone());

    let (queue_sender, queue_receiver) = mpsc::channel::<Message>(100);

    let shutdown = CancellationToken::new();
    let smtp_server = SmtpServer::new(
        smtp_socket,
        "cert.pem".into(),
        "key.pem".into(),
        user_repository,
        queue_sender,
        shutdown.clone(),
    );

    let message_handler = Handler::new(pool.clone(), shutdown.clone());

    smtp_server.spawn();
    message_handler.spawn(queue_receiver);
    shutdown
}

pub async fn run_api_server(pool: PgPool, http_socket: SocketAddrV4) -> CancellationToken {
    let shutdown = CancellationToken::new();
    let api_server = ApiServer::new(http_socket.into(), pool.clone(), shutdown.clone()).await;
    api_server.spawn();
    shutdown
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
