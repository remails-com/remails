use anyhow::Context;
use remails::{run_api_server, shutdown_signal};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

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

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options =
        PgConnectOptions::from_url(&database_url)?.application_name("remails-management");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let http_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3000);

    let shutdown = CancellationToken::new();
    run_api_server(pool, http_socket, shutdown.clone(), true).await;

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    // give services the opportunity to shut down
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
