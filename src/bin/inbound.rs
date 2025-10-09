use anyhow::Context;
use remails::{SmtpConfig, SmtpServer, bus::client::BusClient, init_tracing, shutdown_signal};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options = PgConnectOptions::from_url(&database_url)?.application_name("remails-inbound");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let shutdown = CancellationToken::new();
    let smtp_config = SmtpConfig::default();
    let bus_client = BusClient::new_from_env_var().unwrap();

    let smtp_server = SmtpServer::new(
        pool.clone(),
        Arc::new(smtp_config),
        bus_client.clone(),
        shutdown.clone(),
    );
    smtp_server.spawn();

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    // give services the opportunity to shut down
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
