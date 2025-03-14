use axum::{
    Json, Router,
    extract::{FromRef, State},
    routing::get,
};
use memory_serve::{MemoryServe, load_assets};
use serde::Serialize;
use sqlx::PgPool;
use std::{net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};

use crate::{
    api::oauth::GithubOauthService, message::MessageRepository,
    smtp_credential::SmtpCredentialRepository,
};

mod auth;
mod error;
mod messages;
mod oauth;
mod smtp_credentials;
mod whoami;

#[derive(Debug, Error)]
pub enum ApiServerError {
    #[error("failed to bind to address: {0}")]
    Bind(std::io::Error),
    #[error("server error: {0}")]
    Serve(std::io::Error),
}

#[derive(Clone)]
pub struct ApiState {
    pool: PgPool,
    oauth_service: GithubOauthService,
}

impl FromRef<ApiState> for PgPool {
    fn from_ref(state: &ApiState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<ApiState> for MessageRepository {
    fn from_ref(state: &ApiState) -> Self {
        MessageRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for SmtpCredentialRepository {
    fn from_ref(state: &ApiState) -> Self {
        SmtpCredentialRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for GithubOauthService {
    fn from_ref(state: &ApiState) -> Self {
        state.oauth_service.clone()
    }
}

pub struct ApiServer {
    router: Router,
    socket: SocketAddr,
    shutdown: CancellationToken,
}

impl ApiServer {
    pub async fn new(socket: SocketAddr, pool: PgPool, shutdown: CancellationToken) -> ApiServer {
        let github_oauth = GithubOauthService::new(None).unwrap();
        let oauth_router = github_oauth.router();
        let state = ApiState {
            pool,
            oauth_service: github_oauth,
        };

        let router = Router::new()
            .route("/whoami", get(whoami::whoami))
            .route("/healthy", get(healthy))
            .route("/messages", get(messages::list_messages))
            .route("/messages/{id}", get(messages::get_message))
            .route(
                "/users",
                get(smtp_credentials::list_smtp_credential)
                    .post(smtp_credentials::create_smtp_credential),
            )
            .merge(oauth_router)
            .layer((
                TraceLayer::new_for_http(),
                TimeoutLayer::new(Duration::from_secs(10)),
            ))
            .with_state(state);

        ApiServer {
            socket,
            router: Router::new().nest("/api", router),
            shutdown,
        }
    }

    /// Serve the frontend from the `frontend/dist` directory
    pub async fn serve_frontend(self) -> Self {
        let memory_router = MemoryServe::new(load_assets!("./frontend/dist"))
            .index_file(Some("/index.html"))
            .into_router();

        let router = self.router.merge(memory_router);

        ApiServer {
            socket: self.socket,
            router,
            shutdown: self.shutdown,
        }
    }

    pub async fn serve(self) -> Result<(), ApiServerError> {
        let listener = TcpListener::bind(self.socket)
            .await
            .map_err(ApiServerError::Bind)?;

        info!("API server listening on {}", self.socket);

        axum::serve(
            listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
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
