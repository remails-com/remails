use anyhow::Context;
use remails::{
    HandlerConfig, Kubernetes, SmtpConfig,
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
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    let kubernetes = Kubernetes::new(pool.clone()).await.unwrap();

    // Run retry service
    let periodically = Periodically::new(pool.clone(), bus_client, DnsResolver::default())
        .await
        .unwrap();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = periodically.retry_messages().await {
                error!("Error retrying: {e}")
            };
            if let Err(e) = periodically.clean_up().await {
                error!("Error during clean up: {e}")
            }
            if let Err(e) = kubernetes.check_node_health().await {
                error!("Error during k8s node check: {e}")
            };
        }
    });

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    // give services the opportunity to shut down
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
