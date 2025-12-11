use crate::{
    Environment,
    api::{
        error::AppError,
        messages::create_message_router,
        oauth::GithubOauthService,
        openapi::{docs_router, openapi_router},
    },
    bus::client::BusClient,
    handler::{RetryConfig, dns::DnsResolver},
    models::{
        ApiKeyRepository, ApiUserRepository, DomainRepository, InviteRepository, MessageRepository,
        OrganizationRepository, ProjectRepository, RuntimeConfigRepository,
        SmtpCredentialRepository,
    },
    moneybird::MoneyBird,
};
use axum::{
    BoxError, Json, RequestExt, Router,
    error_handling::HandleErrorLayer,
    extract::{ConnectInfo, FromRef, Request, State},
    middleware,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use base64ct::Encoding;
use http::{
    HeaderName, HeaderValue, Method, StatusCode,
    header::{
        ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONNECTION, COOKIE, HOST, ORIGIN, REFERER,
        USER_AGENT,
    },
};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing::{
    Instrument, Level, error, field, info,
    log::{trace, warn},
    span,
};
use utoipa::ToSchema;

mod api_keys;
mod api_users;
mod auth;
pub mod domains;
mod error;
mod invites;
mod messages;
mod oauth;
pub mod openapi;
mod organizations;
mod projects;
mod smtp_credentials;
mod subscriptions;
mod system;
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

#[derive(Clone, Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct RemailsConfig {
    pub version: String,
    pub api_server_name: String,
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
        let api_server_name = env::var("API_SERVER_NAME").expect("API_SERVER_NAME not set");
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
            api_server_name,
            environment,
            smtp_domain_name,
            smtp_ports,
            spf_include,
            dkim_selector,
            moneybird_administration_id,
        }
    }
}

#[derive(derive_more::Debug)]
pub struct ApiConfig {
    #[debug("****")]
    session_key: cookie::Key,
    pub remails_config: RemailsConfig,
}

#[derive(FromRef, Clone)]
pub struct ApiState {
    pool: PgPool,
    config: Arc<ApiConfig>,
    moneybird: MoneyBird,
    gh_oauth_service: GithubOauthService,
    resolver: DnsResolver,
    message_bus: Arc<BusClient>,
    pub retry_config: Arc<RetryConfig>,
}

impl ApiState {
    pub fn api_server_name(&self) -> &str {
        self.config.remails_config.api_server_name.as_str()
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

impl FromRef<ApiState> for DomainRepository {
    fn from_ref(state: &ApiState) -> Self {
        DomainRepository::new(state.pool.clone())
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

impl FromRef<ApiState> for RuntimeConfigRepository {
    fn from_ref(state: &ApiState) -> Self {
        RuntimeConfigRepository::new(state.pool.clone())
    }
}

impl FromRef<ApiState> for RemailsConfig {
    fn from_ref(state: &ApiState) -> Self {
        state.config.remails_config.clone()
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

fn cors_layer(api_server_name: &str) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(
            format!("https://docs.{}", api_server_name)
                .parse::<HeaderValue>()
                .expect("Could not parse CORS allow origin"),
        )
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            AUTHORIZATION,
            COOKIE,
            ACCEPT,
            ACCEPT_ENCODING,
            USER_AGENT,
            CONNECTION,
            HOST,
            ORIGIN,
            REFERER,
            HeaderName::from_static("priority"),
        ])
}

pub struct ApiServer {
    router: Router,
    socket: SocketAddr,
    shutdown: CancellationToken,
    api_state: ApiState,
}

impl ApiServer {
    pub async fn new(
        socket: SocketAddr,
        pool: PgPool,
        shutdown: CancellationToken,
        with_frontend: bool,
        with_docs: bool,
        message_bus: BusClient,
    ) -> ApiServer {
        let github_oauth = GithubOauthService::new(pool.clone()).unwrap();
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
            config: Arc::new(ApiConfig {
                session_key,
                remails_config: Default::default(),
            }),
            moneybird,
            gh_oauth_service: github_oauth,
            #[cfg(not(test))]
            resolver: DnsResolver::new(),
            #[cfg(test)]
            resolver: DnsResolver::mock("localhost", 0),
            message_bus: Arc::new(message_bus),
            retry_config: Arc::new(RetryConfig::default()),
        };

        let (router, _) = openapi_router().split_for_parts();

        let mut router = router
            .merge(oauth_router)
            .layer((
                TraceLayer::new_for_http(),
                middleware::from_fn(ip_middleware),
            ))
            .layer(cors_layer(&state.config.remails_config.api_server_name))
            .with_state(state.clone());

        if with_frontend {
            let memory_router = memory_serve::from_local_build!()
                .index_file(Some("/index.html"))
                .fallback(Some("/index.html"))
                .fallback_status(StatusCode::OK)
                .into_router();

            router = router.merge(memory_router);
        }

        if with_docs {
            router = router
                .nest("/docs/", docs_router())
                .route("/docs", get(async || Redirect::permanent("/docs/")))
        }

        // Set a hard limit to the size of all requests.
        // Currently, API endpoint to create a new message is the only that allows a larger payload
        // of about 1,2 MB
        router = router
            .layer(RequestBodyLimitLayer::new(12_000))
            .layer(middleware::from_fn(|req, next: Next| async move {
                // TODO I'd prefer a more clean solution for catching errors produced by the
                //  RequestBodyLimitLayer, but could not find any. Also note the [`ValidatedJson`]
                let res = next.run(req).await;
                if res.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    return AppError::PayloadTooLarge.into_response();
                }
                res
            }));

        router = router.merge(
            create_message_router()
                .split_for_parts()
                .0
                .layer((
                    TraceLayer::new_for_http(),
                    middleware::from_fn(ip_middleware),
                ))
                .layer(cors_layer(&state.config.remails_config.api_server_name))
                .with_state(state.clone()),
        );

        router = router.layer(middleware::from_fn_with_state(
            state.config.clone(),
            append_default_headers,
        ));

        router = router.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_timeout_error))
                .timeout(Duration::from_secs(10)),
        );

        ApiServer {
            socket,
            router,
            shutdown,
            api_state: state,
        }
    }

    pub fn api_state(&self) -> &ApiState {
        &self.api_state
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

async fn append_default_headers(
    State(config): State<Arc<ApiConfig>>,
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

async fn handle_timeout_error(err: BoxError) -> AppError {
    if err.is::<tower::timeout::error::Elapsed>() {
        AppError::RequestTimeout
    } else {
        AppError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::error::ApiErrorResponse,
        bus::server::Bus,
        models::{ApiUserId, Role},
    };
    use axum::body::Body;
    use http::{Method, header::CONTENT_LENGTH};
    use std::{
        collections::HashMap,
        net::{Ipv4Addr, SocketAddrV4},
    };
    use tower::{ServiceExt, util::Oneshot};

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

        pub fn set_header(&mut self, name: &'static str, value: Option<String>) {
            if let Some(value) = value {
                self.headers.insert(name, value);
            } else {
                self.headers.remove(name);
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

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn test_request_size_limit(pool: PgPool) {
        let mut server = TestServer::new(pool.clone(), None).await;
        server.set_user(Some(
            "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(),
        ));
        server
            .use_api_key(
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                Role::Maintainer,
            )
            .await;

        for (path, method, size_limit, expected) in [
            (
                "/api/healthy",
                Method::GET,
                12_001,
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                "/api/organizations/44729d9f-a7dc-4226-b412-36a7537f5176/projects/3ba14adf-4de1-4fb6-8c20-50cc2ded5462/emails",
                Method::POST,
                1_200_001,
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                "/api/organizations/44729d9f-a7dc-4226-b412-36a7537f5176/projects/3ba14adf-4de1-4fb6-8c20-50cc2ded5462/emails",
                Method::POST,
                1_200_000,
                StatusCode::BAD_REQUEST,
            ),
        ] {
            server.set_header(CONTENT_LENGTH.as_str(), Some(size_limit.to_string()));
            let response = match method {
                Method::GET => server.get(path),
                Method::POST => server.post(path, Body::default()),
                _ => panic!("Unsupported method"),
            }
            .await
            .unwrap();

            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap();
            let msg = String::from_utf8(bytes.to_vec()).unwrap();
            println!("{msg}");
            assert_eq!(status, expected);
            let _: ApiErrorResponse = serde_json::from_str(&msg).unwrap();
        }

        let response = server
            .post(
                "/api/organizations",
                Body::from(format!(r#""name":"{}""#, "a".repeat(12_000))),
            )
            .await
            .unwrap();

        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        let msg = String::from_utf8(bytes.to_vec()).unwrap();
        println!("{msg}");
        let _: ApiErrorResponse = serde_json::from_str(&msg).unwrap();
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }
}
