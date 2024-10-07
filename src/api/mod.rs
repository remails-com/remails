use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use std::{net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

mod auth;
mod users;

#[derive(Debug, Error)]
pub enum ApiServerError {
    #[error("failed to bind to address: {0}")]
    Bind(std::io::Error),
    #[error("server error: {0}")]
    Serve(std::io::Error),
}

pub struct ApiServer {
    router: Router,
    socket: SocketAddr,
    shutdown: CancellationToken,
}

impl ApiServer {
    pub async fn new(socket: SocketAddr, pool: PgPool, shutdown: CancellationToken) -> ApiServer {
        let router = Router::new()
            .route("/healthy", get(healthy))
            .layer((
                TraceLayer::new_for_http(),
                TimeoutLayer::new(Duration::from_secs(10)),
            ))
            .with_state(pool);

        ApiServer {
            socket,
            router,
            shutdown,
        }
    }

    pub async fn serve(self) -> Result<(), ApiServerError> {
        let listener = TcpListener::bind(self.socket)
            .await
            .map_err(ApiServerError::Bind)?;

        info!("API server listening on {}", self.socket);

        axum::serve(listener, self.router)
            .with_graceful_shutdown(wait_for_shutdown(self.shutdown))
            .await
            .map_err(ApiServerError::Serve)
    }

    pub fn spawn(self) {
        tokio::spawn(async {
            let token = self.shutdown.clone();
            if let Err(e) = self.serve().await {
                error!("server error: {:?}", e);
                token.cancel();
                error!("shutting down API server")
            }
        });
    }
}

async fn wait_for_shutdown(token: CancellationToken) {
    token.cancelled().await;
}

#[derive(Debug, Serialize)]
struct HealthyResponse {
    healthy: bool,
    status: &'static str,
}

async fn healthy(State(pool): State<PgPool>) -> Json<HealthyResponse> {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => Json(HealthyResponse {
            healthy: true,
            status: "OK",
        }),
        Err(e) => {
            error!("database error: {:?}", e);

            Json(HealthyResponse {
                healthy: false,
                status: "database error",
            })
        }
    }
}
