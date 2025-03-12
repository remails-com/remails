use anyhow::Context;
use api::ApiServer;
use handler::Handler;
use message::Message;
use smtp::smtp_server::SmtpServer;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{signal, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use user::UserRepository;

mod api;
mod handler;
mod message;
mod smtp;
mod user;

#[cfg(test)]
mod test;

async fn run(
    pool: PgPool,
    smtp_socket: SocketAddrV4,
    http_socket: SocketAddrV4,
) -> CancellationToken {
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

    let api_server = ApiServer::new(http_socket.into(), pool.clone(), shutdown.clone()).await;

    api_server.spawn();
    smtp_server.spawn();
    message_handler.spawn(queue_receiver);

    shutdown
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=trace,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("failed to connect to database")?;

    let smtp_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3025);
    let http_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3000);

    let shutdown = run(pool, smtp_socket, http_socket).await;

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    // give services the opportunity to shut down
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn shutdown_signal(token: CancellationToken) {
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

#[cfg(test)]
mod smtp_test {
    use super::*;
    use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
    use sqlx::PgPool;
    use test::random_port;
    use tokio::task::JoinHandle;
    use tracing_test::traced_test;
    use user::User;

    async fn setup_server(
        pool: PgPool,
    ) -> (
        CancellationToken,
        JoinHandle<()>,
        mpsc::Receiver<Message>,
        u16,
    ) {
        let smtp_port = random_port();
        let user_repository = UserRepository::new(pool);

        let user = User::new("john".into(), "p4ssw0rd".into());
        user_repository.insert(&user).await.unwrap();

        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
        let shutdown = CancellationToken::new();
        let (queue_sender, receiver) = mpsc::channel::<Message>(100);
        let server = SmtpServer::new(
            socket,
            "cert.pem".into(),
            "key.pem".into(),
            user_repository,
            queue_sender,
            shutdown.clone(),
        );

        let server_handle = tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (shutdown, server_handle, receiver, smtp_port)
    }

    #[sqlx::test]
    #[traced_test]
    async fn test_smtp(pool: PgPool) {
        let (shutdown, server_handle, mut receiver, port) = setup_server(pool).await;

        let message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("Jane Doe", "jane@example.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!");

        SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials(("john", "p4ssw0rd"))
            .connect()
            .await
            .unwrap()
            .send(message)
            .await
            .unwrap();

        shutdown.cancel();
        server_handle.await.unwrap();

        let received_message = receiver.recv().await.unwrap();
        assert_eq!(received_message.get_from(), "john@example.com");
    }

    #[sqlx::test]
    #[traced_test]
    async fn test_smtp_wrong_credentials(pool: PgPool) {
        let (shutdown, server_handle, _, port) = setup_server(pool).await;

        let result = SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials(("john", "wrong"))
            .connect()
            .await;

        assert!(result.is_err());

        shutdown.cancel();
        server_handle.await.unwrap();
    }
}
