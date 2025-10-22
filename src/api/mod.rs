use crate::{
    Environment,
    api::{
        api_keys::{create_api_key, list_api_keys, remove_api_key, update_api_key},
        api_users::{
            delete_password, delete_totp_code, finish_enroll_totp, start_enroll_totp, totp_codes,
            update_password, update_user,
        },
        auth::{logout, password_login, password_register, totp_login},
        domains::{create_domain, delete_domain, get_domain, list_domains, verify_domain},
        invites::{accept_invite, create_invite, get_invite, get_org_invites, remove_invite},
        messages::{create_message, get_message, list_messages, remove_message, retry_now},
        oauth::GithubOauthService,
        organizations::{
            create_organization, get_organization, list_members, list_organizations, remove_member,
            remove_organization, update_member_role, update_organization,
        },
        projects::{create_project, list_projects, remove_project, update_project},
        smtp_credentials::{
            create_smtp_credential, list_smtp_credential, remove_smtp_credential,
            update_smtp_credential,
        },
        streams::{create_stream, list_streams, remove_stream, update_stream},
        subscriptions::{get_sales_link, get_subscription, moneybird_webhook},
    },
    bus::client::BusClient,
    handler::dns::DnsResolver,
    models::{
        ApiKeyRepository, ApiUserRepository, DomainRepository, InviteRepository, MessageRepository,
        OrganizationRepository, ProjectRepository, SmtpCredentialRepository, StreamRepository,
    },
    moneybird::MoneyBird,
};
use axum::{
    Json, RequestExt, Router,
    extract::{ConnectInfo, FromRef, Request, State},
    middleware,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use base64ct::Encoding;
use http::{HeaderName, HeaderValue, StatusCode};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr, time::Duration};
use thiserror::Error;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{
    Instrument, Level, error, field, info,
    log::{trace, warn},
    span,
};

mod api_keys;
mod api_users;
mod auth;
pub mod domains;
mod error;
mod invites;
mod messages;
mod oauth;
mod organizations;
mod projects;
mod smtp_credentials;
mod streams;
mod subscriptions;
mod validation;
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
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct RemailsConfig {
    pub version: String,
    pub environment: Environment,
    pub smtp_domain_name: String,
    pub smtp_ports: Vec<u16>,
    pub spf_include: String,
    pub dkim_selector: String,
    pub moneybird_administration_id: String,
}

impl Default for RemailsConfig {
    fn default() -> Self {
        let version = env::var("VERSION").unwrap_or("dev".to_string());
        let environment: Environment = env::var("ENVIRONMENT")
            .map(|s| s.parse())
            .inspect_err(|_| {
                tracing::warn!("Did not find ENVIRONMENT env var, defaulting to development")
            })
            .unwrap_or(Ok(Environment::Development))
            .expect(
                "Invalid ENVIRONMENT env var, must be one of: development, production, or staging",
            );
        let smtp_domain_name =
            env::var("SMTP_SERVER_NAME").expect("SMTP_SERVER_NAME env var must be set");
        let smtp_ports = env::var("SMTP_PORTS")
            .map(|s| {
                s.split(",")
                    .map(|p| p.parse().expect("Could not parse port"))
                    .collect()
            })
            .expect("SMTP_PORTS env var must be set");
        let spf_include = env::var("SPF_INCLUDE").unwrap_or("include:spf.remails.net".to_string());

        let dkim_selector = env::var("DKIM_SELECTOR").expect("DKIM_SELECTOR env var must be set");
        let moneybird_administration_id = env::var("MONEYBIRD_ADMINISTRATION_ID")
            .expect("MONEYBIRD_ADMINISTRATION_ID env var must be set");

        Self {
            version,
            environment,
            smtp_domain_name,
            smtp_ports,
            spf_include,
            dkim_selector,
            moneybird_administration_id,
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
    message_bus: BusClient,
}

impl FromRef<ApiState> for PgPool {
    fn from_ref(state: &ApiState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<ApiState> for BusClient {
    fn from_ref(state: &ApiState) -> Self {
        state.message_bus.clone()
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

impl FromRef<ApiState> for ApiKeyRepository {
    fn from_ref(state: &ApiState) -> Self {
        ApiKeyRepository::new(state.pool.clone())
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

impl FromRef<ApiState> for InviteRepository {
    fn from_ref(state: &ApiState) -> Self {
        InviteRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for (InviteRepository, OrganizationRepository) {
    fn from_ref(state: &ApiState) -> Self {
        (
            InviteRepository::new(state.pool.clone()),
            OrganizationRepository::new(state.pool.clone()),
        )
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

async fn ip_middleware(mut request: Request, next: Next) -> Response {
    let span = span!(
        Level::INFO,
        "ip_addr",
        real_ip = field::Empty,
        connection_ip = field::Empty
    );

    let real_ip = request.headers().get("x-forwarded-for");
    if let Some(real_ip) = real_ip {
        span.record("real_ip", real_ip.to_str().unwrap_or("unknown"));
    }

    let connection_ip = request.extract_parts::<ConnectInfo<SocketAddr>>().await;
    if let Ok(connection_ip) = connection_ip {
        span.record("connection_ip", connection_ip.to_string());
    }

    next.run(request).instrument(span).await
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
        message_bus: BusClient,
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
            message_bus,
        };

        let mut router = Router::new()
            .route("/config", get(config))
            .route("/whoami", get(whoami::whoami))
            .route("/healthy", get(healthy))
            .route("/webhook/moneybird", post(moneybird_webhook))
            .route("/api_user/{user_id}", put(update_user))
            .route("/api_user/{user_id}/password", put(update_password).delete(delete_password))
            .route("/api_user/{user_id}/totp/enroll", get(start_enroll_totp).post(finish_enroll_totp))
            .route("/api_user/{user_id}/totp", get(totp_codes))
            .route("/api_user/{user_id}/totp/{totp_id}", delete(delete_totp_code))
            .route(
                "/organizations",
                get(list_organizations).post(create_organization),
            )
            .route(
                "/organizations/{id}",
                get(get_organization).put(update_organization).delete(remove_organization),
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
                "/organizations/{org_id}/projects",
                get(list_projects).post(create_project),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}",
                delete(remove_project).put(update_project),
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
                get(list_messages).post(create_message),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages/{message_id}",
                get(get_message).delete(remove_message),
            )
            .route(
                "/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages/{message_id}/retry",
                put(retry_now),
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
            .route(
                "/organizations/{org_id}/members",
                get(list_members),
            )
            .route(
                "/organizations/{org_id}/members/{user_id}",
                delete(remove_member).put(update_member_role),
            )
            .route(
                "/organizations/{org_id}/api_keys",
                get(list_api_keys).post(create_api_key),
            )
            .route(
                "/organizations/{org_id}/api_keys/{api_key_id}",
                delete(remove_api_key).put(update_api_key),
            )
            .route("/logout", get(logout))
            .route("/login/password", post(password_login))
            .route("/login/totp", post(totp_login))
            .route("/register/password", post(password_register))
            .route("/invite/{org_id}", get(get_org_invites).post(create_invite))
            .route("/invite/{org_id}/{invite_id}", delete(remove_invite))
            .route("/invite/{org_id}/{invite_id}/{password}", get(get_invite).post(accept_invite))
            .fallback(api_fallback)
            .merge(oauth_router)
            .layer((
                TraceLayer::new_for_http(),
                middleware::from_fn(ip_middleware),
                TimeoutLayer::new(Duration::from_secs(10)),
            ))
            .with_state(state.clone());

        router = Router::new().nest("/api", router);

        if with_frontend {
            let memory_router = memory_serve::from_local_build!()
                .index_file(Some("/index.html"))
                .fallback(Some("/index.html"))
                .fallback_status(StatusCode::OK)
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

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async {
            let token = self.shutdown.clone();
            if let Err(e) = self.serve().await {
                error!("server error: {:?}", e);
                token.cancel();
                error!("shutting down API server")
            }
        })
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

pub async fn config(State(config): State<RemailsConfig>) -> Response {
    Json(config).into_response()
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

#[cfg(test)]
mod tests {
    use crate::{bus::server::Bus, models::ApiUserId};
    use axum::body::Body;
    use http::Method;
    use std::{
        collections::HashMap,
        net::{Ipv4Addr, SocketAddrV4},
    };
    use tower::{ServiceExt, util::Oneshot};

    use super::*;

    pub struct TestServer {
        server: ApiServer,
        pub message_bus: BusClient,
        pub headers: HashMap<&'static str, String>,
    }

    impl TestServer {
        pub async fn new(pool: PgPool, user: Option<ApiUserId>) -> Self {
            let http_socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0);
            let shutdown = CancellationToken::new();
            let message_bus_port = Bus::spawn_random_port().await;
            let message_bus_client =
                BusClient::new(message_bus_port, "localhost".to_string()).unwrap();
            let server = ApiServer::new(
                http_socket.into(),
                pool.clone(),
                shutdown,
                false,
                message_bus_client.clone(),
            )
            .await;
            let mut headers = HashMap::new();
            headers.insert("Content-Type", "application/json".to_string());
            if let Some(user) = user {
                headers.insert("X-Test-Login-ID", user.to_string());
            }
            TestServer {
                server,
                headers,
                message_bus: message_bus_client,
            }
        }

        pub fn set_user(&mut self, user: Option<ApiUserId>) {
            if let Some(user) = user {
                self.headers.insert("X-Test-Login-ID", user.to_string());
            } else {
                self.headers.remove("X-Test-Login-ID");
            }
        }

        fn request<U: AsRef<str>>(
            &self,
            method: Method,
            uri: U,
            body: Body,
        ) -> Oneshot<Router, Request<Body>> {
            let mut request = Request::builder().method(method).uri(uri.as_ref());

            for (&name, value) in self.headers.iter() {
                request = request.header(name, value);
            }

            let request = request.body(body).unwrap();

            self.server.router.clone().oneshot(request)
        }

        pub fn get<U: AsRef<str>>(&self, uri: U) -> Oneshot<Router, Request<Body>> {
            self.request(Method::GET, uri, Body::empty())
        }

        pub fn post<U: AsRef<str>>(&self, uri: U, body: Body) -> Oneshot<Router, Request<Body>> {
            self.request(Method::POST, uri, body)
        }

        pub fn put<U: AsRef<str>>(&self, uri: U, body: Body) -> Oneshot<Router, Request<Body>> {
            self.request(Method::PUT, uri, body)
        }

        pub fn delete<U: AsRef<str>>(&self, uri: U) -> Oneshot<Router, Request<Body>> {
            self.request(Method::DELETE, uri, Body::empty())
        }

        pub fn delete_with_body<U: AsRef<str>>(
            &self,
            uri: U,
            body: Body,
        ) -> Oneshot<Router, Request<Body>> {
            self.request(Method::DELETE, uri, body)
        }
    }

    pub fn serialize_body<T>(body: T) -> Body
    where
        T: serde::Serialize,
    {
        let json = serde_json::to_string(&body).unwrap();
        Body::from(json)
    }

    pub async fn deserialize_body<T>(body: Body) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        let bytes = axum::body::to_bytes(body, 8192).await.unwrap();
        serde_json::from_slice(&bytes).expect("Failed to deserialize response body")
    }

    #[sqlx::test]
    async fn test_util_endpoints(pool: PgPool) {
        let server = TestServer::new(pool.clone(), None).await;

        // can access health check
        let response = server.get("/api/healthy").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        assert!(bytes.iter().eq(b"{\"healthy\":true,\"status\":\"OK\"}"));

        // can access Remails config
        let response = server.get("/api/config").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _: RemailsConfig = deserialize_body(response.into_body()).await;
    }
}
