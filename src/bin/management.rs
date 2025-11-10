use anyhow::Context;
use futures::future;
use remails::{
    api::openapi::spawn_docs, bus::client::BusClient, init_tracing, run_api_server, shutdown_signal,
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
        PgConnectOptions::from_url(&database_url)?.application_name("remails-management");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("failed to connect to database")?;

    let api_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3000);

    let shutdown = CancellationToken::new();
    let bus = BusClient::new_from_env_var().expect("Could not connect to message bus");
    let api_join = run_api_server(pool, bus, api_socket, shutdown.clone(), true, false).await;

    let docs_socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3001);
    let docs_join = spawn_docs(docs_socket.into(), shutdown.clone());

    shutdown_signal(shutdown.clone()).await;
    info!("received shutdown signal, stopping services");
    shutdown.cancel();

    tokio::select!(
        // gracefully shutdown
        _ = future::try_join_all([api_join, docs_join]) => {
            info!("Shut down");
        }
        // hard shutdown if it takes more than 2 secs
        _ = tokio::time::sleep(Duration::from_secs(2)) => {
            warn!("stopping services takes too long, hard shutdown");
        }
    );

    Ok(())
}
