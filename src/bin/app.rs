use anyhow::Context;
use remails::{
    HandlerConfig, SmtpConfig,
    bus::{client::BusClient, server::Bus},
    handler::dns::DnsResolver,
    periodically::Periodically,
    run_api_server, run_mta, shutdown_signal,
};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// This is the main function used for local testing
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("remails=trace,tower_http=debug,axum=trace".parse().unwrap()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .without_time(),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?
        .parse()
        .expect("DATABASE_URL must be a valid URL");

    let db_options =
        PgConnectOptions::from_url(&database_url)?.application_name("remails-all-in-one-app");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    #[cfg(feature = "apply-db-migrations")]
    sqlx::migrate!().run(&pool).await?;

    let api_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3000);
    let smtp_config = SmtpConfig::default();
    let handler_config = HandlerConfig::new();
    let shutdown = CancellationToken::new();
    let bus_client = BusClient::new_from_env_var().unwrap();
    let bus_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 4000);

    // Run message bus
    tokio::spawn(async move {
        let (tx, _rx) = tokio::sync::broadcast::channel::<String>(100);
        let bus = Bus::new(bus_socket, tx);
        bus.serve().await
    });

    run_mta(
        pool.clone(),
        smtp_config,
        handler_config.clone(),
        bus_client.clone(),
        shutdown.clone(),
    )
    .await;
    run_api_server(
        pool.clone(),
        bus_client.clone(),
        api_socket,
        shutdown.clone(),
        true,
        true,
    )
    .await;

    // Run retry service
    let api_server_name = std::env::var("API_SERVER_NAME").expect("API_SERVER_NAME must be set");
    let periodically = Periodically::new(
        pool.clone(),
        bus_client,
        DnsResolver::default(),
        api_server_name,
    )
    .await
    .unwrap();

    let shutdown_clone = shutdown.clone();
    let join_handle = tokio::spawn(async move {
        let get_interval = |secs: u64| {
            let mut interval = tokio::time::interval(Duration::from_secs(secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval
        };
        let mut retry_interval = get_interval(10); // Every 10 seconds
        let mut verify_domains_interval = get_interval(60); // Every minute
        let mut reset_quotas_interval = get_interval(120); // Every 2 minutes
        let mut block_orgs_interval = get_interval(120); // Every 2 minutes
        let mut clean_up_interval = get_interval(300); // Every 5 minutes

        loop {
            tokio::select! {
                _ = retry_interval.tick() => {
                    if let Err(e) = periodically.retry_messages().await {
                        error!("Error retrying messages: {e}")
                    }
                },
                _ = verify_domains_interval.tick() => {
                    if let Err(e) = periodically.verify_domains().await {
                        error!("Error verifying domains: {e}")
                    }
                },
                _ = reset_quotas_interval.tick() => {
                    if let Err(e) = periodically.reset_all_quotas().await {
                        error!("Error resetting quotas: {e}")
                    }
                },
                _ = block_orgs_interval.tick() => {
                    if let Err(e) = periodically.block_suspicious_orgs().await {
                        error!("Error blocking suspicious orgs: {e}")
                    }
                },
                _ = clean_up_interval.tick() => {
                    if let Err(e) = periodically.clean_up().await {
                        error!("Error during clean up: {e}")
                    }
                },
                _ = shutdown_clone.cancelled() => break,
            }
        }
    });

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    tokio::select! {
        _ = join_handle => {
            info!("Shut down");
        }
        _ = tokio::time::sleep(Duration::from_secs(2)) => {
            warn!("stopping services takes too long, hard shutdown");
        }
    }

    Ok(())
}
