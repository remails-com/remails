use crate::{
    Environment,
    api::{
        auth::{logout, password_login, password_register},
        domains::{create_domain, delete_domain, get_domain, list_domains, verify_domain},
        messages::{get_message, list_messages, remove_message, update_to_retry_asap},
        oauth::GithubOauthService,
        organizations::{
            create_organization, get_organization, list_organizations, remove_organization,
        },
        projects::{create_project, list_projects, remove_project, update_project},
        smtp_credentials::{
            create_smtp_credential, list_smtp_credential, remove_smtp_credential,
            update_smtp_credential,
        },
        streams::{create_stream, list_streams, remove_stream, update_stream},
        subscriptions::{get_sales_link, get_subscription, moneybird_webhook},
    },
    handler::dns::DnsResolver,
    models::{
        ApiUserRepository, DomainRepository, MessageRepository, OrganizationRepository,
        ProjectRepository, SmtpCredentialRepository, StreamRepository,
    },
    moneybird::MoneyBird,
};
use axum::{
    Json, Router,
    extract::{FromRef, Request, State},
    middleware,
    routing::{delete, get, post, put},
};
use base64ct::Encoding;
use http::{HeaderName, HeaderValue, StatusCode};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{
    error, info,
    log::{trace, warn},
};

mod api_users;
mod auth;
mod config;
pub mod domains;
mod error;
mod messages;
mod oauth;
mod organizations;
mod projects;
mod smtp_credentials;
mod streams;
mod subscriptions;
mod whoami;

static USER_AGENT_VALUE: &str = "remails";

#[derive(Debug, Error)]
pub enum ApiServerError {
    #[error("failed to bind to address: {0}")]
    Bind(std::io::Error),
    #[error("server error: {0}")]
    Serve(std::io::Error),
}

#[derive(Clone, Debug, Serialize)]
pub struct RemailsConfig {
    pub version: String,
    pub environment: Environment,
    pub smtp_domain_name: String,
    pub smtp_ports: Vec<u16>,
    pub preferred_spf_record: String,
    pub dkim_selector: String,
}

impl Default for RemailsConfig {
    fn default() -> Self {
        let version = env::var("VERSION").unwrap_or("dev".to_string());
        let environment: Environment = env::var("ENVIRONMENT")
            .map(|s| s.parse())
            .unwrap_or(Ok(Environment::Development))
            .unwrap_or(Environment::Development);
        let smtp_domain_name =
            env::var("SMTP_SERVER_NAME").expect("SMTP_SERVER_NAME env var must be set");
        let smtp_ports = env::var("SMTP_PORTS")
            .map(|s| {
                s.split(",")
                    .map(|p| p.parse().expect("Could not parse port"))
                    .collect()
            })
            .expect("SMTP_PORTS env var must be set");
        let preferred_spf_record = env::var("PREFERRED_SPF_RECORD")
            .unwrap_or("v=spf1 include:spf.remails.net -all".to_string());

        let dkim_selector = env::var("DKIM_SELECTOR").expect("DKIM_SELECTOR env var must be set");

        Self {
            version,
            environment,
            smtp_domain_name,
            smtp_ports,
            preferred_spf_record,
            dkim_selector,
        }
    }
}

#[derive(Clone, derive_more::Debug)]
pub struct ApiConfig {
    #[debug("****")]
    session_key: cookie::Key,
    pub remails_config: RemailsConfig,
}

#[derive(Clone)]
pub struct ApiState {
    pool: PgPool,
    config: ApiConfig,
    moneybird: MoneyBird,
    gh_oauth_service: GithubOauthService,
    resolver: DnsResolver,
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

impl FromRef<ApiState> for ProjectRepository {
    fn from_ref(state: &ApiState) -> Self {
        ProjectRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for StreamRepository {
    fn from_ref(state: &ApiState) -> Self {
        StreamRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for DomainRepository {
    fn from_ref(state: &ApiState) -> Self {
        DomainRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for (DomainRepository, DnsResolver, RemailsConfig) {
    fn from_ref(state: &ApiState) -> Self {
        (
            DomainRepository::new(state.pool.clone()),
            state.resolver.clone(),
            state.config.remails_config.clone(),
        )
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

impl FromRef<ApiState> for RemailsConfig {
    fn from_ref(state: &ApiState) -> Self {
        state.config.remails_config.clone()
    }
}

impl FromRef<ApiState> for MoneyBird {
    fn from_ref(state: &ApiState) -> Self {
        state.moneybird.clone()
    }
}

async fn api_fallback() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "status": "Not Found" })),
    )
}

pub struct ApiServer {
    router: Router,
    socket: SocketAddr,
    shutdown: CancellationToken,
}

impl ApiServer {
    pub async fn new(
        socket: SocketAddr,
        pool: PgPool,
        shutdown: CancellationToken,
        with_frontend: bool,
    ) -> ApiServer {
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

        let moneybird = MoneyBird::new(pool.clone())
            .await
            .expect("Cannot connect to Moneybird");

        moneybird.register_webhook();

        let state = ApiState {
            pool,
            config: ApiConfig {
                session_key,
                remails_config: Default::default(),
            },
            moneybird,
            gh_oauth_service: github_oauth,
            #[cfg(not(test))]
            resolver: DnsResolver::new(),
            #[cfg(test)]
            resolver: DnsResolver::mock("localhost", 0),
        };

        let mut router = Router::new()
            .route("/config", get(config::config))
            .route("/whoami", get(whoami::whoami))
            .route("/healthy", get(healthy))
            .route("/webhook/moneybird", post(moneybird_webhook))
            .route("/api_user/{user_id}", put(api_users::update_user))
            .route("/api_user/{user_id}/password", put(api_users::update_password).delete(api_users::delete_password))
            .route(
                "/organizations",
                get(list_organizations).post(create_organization),
            )
            .route(
                "/organizations/{id}",
                get(get_organization).delete(remove_organization),
            )
            .route(
                "/organizations/{id}/subscription",
                get(get_subscription),
            )
            .route(
                "/organizations/{id}/subscription/new",
                get(get_sales_link),
            )
            .route(
                "/organizations/{org_id}/messages",
                get(list_messages),
            )
            .route(
                "/organizations/{org_id}/messages/{message_id}",
                get(get_message).delete(remove_message),
            )
            .route(
                "/organizations/{org_id}/messages/{message_id}/retry",
                put(update_to_retry_asap),
            )
            .route(
                "/organizations/{org_id}/projects",
                get(list_projects).post(create_project),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}",
                delete(remove_project).put(update_project),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/messages",
                get(list_messages),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/messages/{message_id}",
                get(get_message).delete(remove_message),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/messages/{message_id}/retry",
                put(update_to_retry_asap),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams",
                get(list_streams).post(create_stream),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}",
                delete(remove_stream).put(update_stream),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials",
                get(list_smtp_credential).post(create_smtp_credential),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials/{credential_id}",
                delete(remove_smtp_credential).put(update_smtp_credential),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages",
                get(list_messages),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages/{message_id}",
                get(get_message).delete(remove_message),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages/{message_id}/retry",
                put(update_to_retry_asap),
            )
            .route(
                "/organizations/{org_id}/domains",
                get(list_domains).post(create_domain),
            )
            .route(
                "/organizations/{org_id}/domains/{domain_id}",
                get(get_domain).delete(delete_domain),
            )
            .route(
                "/organizations/{org_id}/domains/{domain_id}/verify",
                post(verify_domain),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/domains",
                get(list_domains).post(create_domain),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/domains/{domain_id}",
                get(get_domain).delete(delete_domain),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/domains/{domain_id}/verify",
                post(verify_domain),
            )
            .route("/logout", get(logout))
            .route("/login/password", post(password_login))
            .route("/register/password", post(password_register))
            .fallback(api_fallback)
            .merge(oauth_router)
            .layer((
                TraceLayer::new_for_http(),
                TimeoutLayer::new(Duration::from_secs(10)),
            ))
            .with_state(state.clone());

        router = Router::new().nest("/api", router);

        if with_frontend {
            let memory_router = memory_serve::from_local_build!()
                .index_file(Some("/index.html"))
                .fallback(Some("/index.html"))
                .into_router();

            router = router.merge(memory_router);
        }

        router = router.layer(middleware::from_fn_with_state(
            state.config.clone(),
            append_default_headers,
        ));

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

async fn append_default_headers(
    State(config): State<ApiConfig>,
    req: Request,
    next: middleware::Next,
) -> axum::response::Response {
    let mut res = next.run(req).await;

    if !matches!(config.remails_config.environment, Environment::Production) {
        res.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        );
    }

    if let Ok(version_header_value) = HeaderValue::from_str(&config.remails_config.version) {
        res.headers_mut().insert(
            HeaderName::from_static("x-app-version"),
            version_header_value,
        );
    } else {
        trace!("Failed to append version header")
    }

    res
}
