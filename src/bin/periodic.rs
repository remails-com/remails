use anyhow::Context;
use remails::{
    Kubernetes, bus::client::BusClient, init_tracing, periodically::Periodically, shutdown_signal,
};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::time::{self, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options =
        PgConnectOptions::from_url(&database_url)?.application_name("remails-periodic");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let bus_client = BusClient::new_from_env_var()?;
    let periodically = Periodically::new(pool.clone(), bus_client).await?;
    let kubernetes = Kubernetes::new(pool.clone()).await?;

    let shutdown = CancellationToken::new();
    let mut chech_nodes_interval = time::interval(Duration::from_secs(10)); // Every 30 seconds
    let mut message_retry_interval = time::interval(Duration::from_secs(60)); // Every minute
    let mut reset_all_quotas_interval = time::interval(Duration::from_secs(10 * 60)); // Every 10 minutes
    let mut clean_up_invites_interval = time::interval(Duration::from_secs(4 * 60 * 60)); // Every 4 hours
    chech_nodes_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    message_retry_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    reset_all_quotas_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    clean_up_invites_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let shutdown_clone = shutdown.clone();

    let join_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = chech_nodes_interval.tick() => {
                    if let Err(err) = kubernetes.check_node_health().await {
                        error!("Failed to check K8s nodes health: {}", err);
                    }
                },
                _ = message_retry_interval.tick() => {
                    if let Err(err) = periodically.retry_messages().await {
                        error!("Failed to retry messages: {}", err);
                    }
                },
                _ = reset_all_quotas_interval.tick() => {
                    if let Err(err) = periodically.reset_all_quotas().await {
                        error!("Failed to reset all quotas: {}", err);
                    }
                },
                _ = clean_up_invites_interval.tick() =>  {
                    if let Err(err) = periodically.clean_up_invites().await {
                        error!("Failed to clean up invites: {}", err);
                    }
                },
                _ = shutdown_clone.cancelled() => {
                    break
                },
            }
        }
    });

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
