use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration;
use thiserror::Error;
use tokio::{net::TcpListener, signal};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

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
}

impl ApiServer {
    pub async fn new(pool: PgPool) -> ApiServer {
        let router = Router::new()
            .route("/healthy", get(healthy))
            .layer((
                TraceLayer::new_for_http(),
                TimeoutLayer::new(Duration::from_secs(10)),
            ))
            .with_state(pool);

        ApiServer { router }
    }

    pub async fn serve(self) -> Result<(), ApiServerError> {
        let listener = TcpListener::bind("0.0.0.0:3000")
            .await
            .map_err(ApiServerError::Bind)?;

        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(ApiServerError::Serve)
    }

    pub async fn spawn(self) {
        tokio::spawn(async {
            if let Err(e) = self.serve().await {
                tracing::error!("server error: {:?}", e);
            }
        });
    }
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
            tracing::error!("database error: {:?}", e);

            Json(HealthyResponse {
                healthy: false,
                status: "database error",
            })
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
