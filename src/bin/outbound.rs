use anyhow::Context;
use remails::{
    HandlerConfig, bus::client::BusClient, handler::Handler, init_tracing, shutdown_signal,
};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options =
        PgConnectOptions::from_url(&database_url)?.application_name("remails-outbound");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let shutdown = CancellationToken::new();
    let handler_config = HandlerConfig::new();
    let bus_client = BusClient::new_from_env_var()?;

    let message_handler =
        Handler::new(pool, Arc::new(handler_config), bus_client, shutdown.clone()).await;
    let join_handle = message_handler.spawn();

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    tokio::select!(
        // gracefully shutdown
        _ = join_handle => {
            info!("Shut down");
        }
        // hard shutdown if it takes more than 2 secs
        _ = tokio::time::sleep(Duration::from_secs(2)) => {
            warn!("stopping services takes too long, hard shutdown");
        }
    );

    Ok(())
}
