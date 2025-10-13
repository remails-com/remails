use anyhow::Context;
use remails::{bus::client::BusClient, init_tracing, periodically::Periodically};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};

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

    let bus_client = BusClient::new_from_env_var().unwrap();
    let periodically = Periodically::new(pool.clone(), bus_client).await.unwrap();

    // should these run on separate cooldowns?
    periodically.retry_messages().await?; // every minute?
    periodically.reset_all_quotas().await?; // every 10 minutes?
    periodically.clean_up_invites().await?; // every 4 hours?

    // use tokio::select! to run the different functions with sleeps in between executions?
    // I think we want sleeps and not a proper interval in case the operations start taking longer

    Ok(())
}
