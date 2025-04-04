use anyhow::Context;
use remails::{HandlerConfig, SmtpConfig, run_api_server, run_mta, shutdown_signal};
use sqlx::postgres::PgPoolOptions;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "load-fixtures")]
use tracing::error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("remails=trace,tower_http=debug,axum=trace".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("failed to connect to database")?;

    #[cfg(feature = "load-fixtures")]
    if let Err(e) = remails::load_fixtures(&pool).await {
        error!("Failed to load fixtures: {e:?}");
    }

    let http_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3000);
    let smtp_config = SmtpConfig::default();
    // TODO change me
    let handler_config = HandlerConfig::new("remails-dev.tweedegolf.nl");
    let shutdown = CancellationToken::new();

    run_mta(pool.clone(), smtp_config, handler_config, shutdown.clone()).await;
    run_api_server(pool, http_socket, shutdown.clone(), true).await;

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    // give services the opportunity to shut down
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
