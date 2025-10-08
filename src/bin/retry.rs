use anyhow::Context;
use remails::{HandlerConfig, MoneyBird, handler::Handler, init_tracing};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio_util::sync::CancellationToken;
use tracing::error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

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

    let moneybird = MoneyBird::new(pool.clone())
        .await
        .expect("Cannot connect to Moneybird");

    moneybird
        .reset_all_quotas()
        .await
        .inspect_err(|err| error!("Failed to reset quotas: {}", err))
        .ok();

    let shutdown = CancellationToken::new();
    let handler_config = HandlerConfig::new();

    let message_handler = Handler::new(pool.clone(), handler_config.into(), shutdown);

    message_handler.retry_all().await?;
    message_handler.periodic_clean_up().await?;

    Ok(())
}
