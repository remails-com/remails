use crate::{
    api::{
        auth::logout,
        messages::{get_message, list_messages},
        oauth::GithubOauthService,
        organizations::{
            create_organization, get_organization, list_organizations, remove_organization,
        },
        smtp_credentials::{create_smtp_credential, list_smtp_credential},
    },
    models::{
        ApiUserRepository, MessageRepository, OrganizationRepository, SmtpCredentialRepository,
    },
};
use axum::{
    Json, Router,
    extract::{FromRef, State},
    routing::get,
};
use base64ct::Encoding;
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info, log::warn};

mod api_users;
mod auth;
mod error;
mod messages;
mod oauth;
mod organizations;
mod smtp_credentials;
mod whoami;

static USER_AGENT_VALUE: &str = "remails";

#[derive(Debug, Error)]
pub enum ApiServerError {
    #[error("failed to bind to address: {0}")]
    Bind(std::io::Error),
    #[error("server error: {0}")]
    Serve(std::io::Error),
}

#[derive(Clone, derive_more::Debug)]
pub struct ApiConfig {
    #[debug("****")]
    pub session_key: cookie::Key,
}

#[derive(Clone)]
pub struct ApiState {
    pool: PgPool,
    config: ApiConfig,
    gh_oauth_service: GithubOauthService,
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

impl FromRef<ApiState> for OrganizationRepository {
    fn from_ref(state: &ApiState) -> Self {
        OrganizationRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for ApiUserRepository {
    fn from_ref(state: &ApiState) -> Self {
        ApiUserRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for GithubOauthService {
    fn from_ref(state: &ApiState) -> Self {
        state.gh_oauth_service.clone()
    }
}

pub struct ApiServer {
    router: Router,
    socket: SocketAddr,
    shutdown: CancellationToken,
}

impl ApiServer {
    pub async fn new(socket: SocketAddr, pool: PgPool, shutdown: CancellationToken) -> ApiServer {
        let github_oauth = GithubOauthService::new(ApiUserRepository::new(pool.clone())).unwrap();
        let oauth_router = github_oauth.router();

        let session_key = match env::var("SESSION_KEY") {
            Ok(session_key_base64) => {
                let key_bytes = base64ct::Base64::decode_vec(&session_key_base64)
                    .expect("SESSION_KEY env var must be valid base 64");
                cookie::Key::from(&key_bytes)
            }
            Err(_) => {
                warn!("Could not find SESSION_KEY; generating one");
                cookie::Key::generate()
            }
        };

        let state = ApiState {
            pool,
            config: ApiConfig { session_key },
            gh_oauth_service: github_oauth,
        };

        let router = Router::new()
            .route("/whoami", get(whoami::whoami))
            .route("/healthy", get(healthy))
            .route("/messages", get(list_messages))
            .route("/messages/{id}", get(get_message))
            .route(
                "/smtp_credentials",
                get(list_smtp_credential).post(create_smtp_credential),
            )
            .route(
                "/organizations",
                get(list_organizations).post(create_organization),
            )
            .route(
                "/organizations/{id}",
                get(get_organization).delete(remove_organization),
            )
            .route("/logout", get(logout))
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
        let memory_router = memory_serve::from_local_build!()
            .index_file(Some("/index.html"))
            .fallback(Some("/index.html"))
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
