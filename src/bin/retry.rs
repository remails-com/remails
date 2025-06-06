use anyhow::Context;
use remails::{HandlerConfig, handler::Handler};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("remails=trace,tower_http=debug,axum=trace".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options = PgConnectOptions::from_url(&database_url)?.application_name("remails-retry");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let shutdown = CancellationToken::new();
    // TODO change me
    let handler_config = HandlerConfig::new("remails.tweedegolf-test.nl");

    let message_handler = Handler::new(pool.clone(), handler_config.into(), shutdown);

    message_handler.retry_all().await?;

    Ok(())
}
