use crate::api::{
    ApiServerError, ApiState, api_fallback, api_keys, api_users, auth, domains,
    domains::{create_domain, delete_domain, get_domain, list_domains, verify_domain},
    error, invites, messages, organizations, projects, smtp_credentials, subscriptions,
    wait_for_shutdown, whoami,
};
use axum::{Json, Router, routing::get};
use memory_serve::{MemoryServe, load_assets};
use std::{env, net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tower_http::{
    compression::CompressionLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer,
};
use tracing::info;
use utoipa::{
    OpenApi,
    openapi::{
        ContactBuilder, SecurityRequirement, Server,
        security::{Http, HttpAuthScheme, SecurityScheme},
    },
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn openapi_router() -> OpenApiRouter<ApiState> {
    let version = env::var("VERSION").unwrap_or("dev".to_string());

    #[derive(utoipa::OpenApi)]
    #[openapi(components(schemas(
        error::ApiErrorResponse,
        crate::models::OrganizationId,
        crate::models::Password,
    )))]
    struct ApiDoc;

    let http_basic = Http::builder()
        .scheme(HttpAuthScheme::Basic)
        .description(Some(
            "You can generate new API keys in the organization setting".to_string(),
        ))
        .build();
    let security = SecurityScheme::Http(http_basic);
    let mut api_doc = ApiDoc::openapi();

    #[cfg(feature = "internal-api-docs")]
    let cookie_auth = SecurityScheme::ApiKey(utoipa::openapi::security::ApiKey::Cookie(
        utoipa::openapi::security::ApiKeyValue::new(auth::SESSION_COOKIE_NAME),
    ));

    let mut components = api_doc.components.unwrap_or_default();
    components.add_security_scheme("basicAuth", security);
    #[cfg(feature = "internal-api-docs")]
    components.add_security_scheme("cookieAuth", cookie_auth);
    api_doc.components = Some(components);
    let mut security = api_doc.security.unwrap_or_default();
    security.push(SecurityRequirement::new::<&str, [_; 0], String>(
        "basicAuth",
        [],
    ));
    api_doc.security = Some(security);

    let api_server_name = env::var("API_SERVER_NAME").expect("API_SERVER_NAME not set");
    #[cfg(debug_assertions)]
    {
        api_doc.servers = Some(vec![Server::new(format!("http://{api_server_name}"))]);
    }
    #[cfg(not(debug_assertions))]
    {
        api_doc.servers = Some(vec![Server::new(format!("https://{api_server_name}"))]);
    }

    let mut router = OpenApiRouter::with_openapi(api_doc).nest(
        "/api",
        OpenApiRouter::default()
            .merge(organizations::router())
            .merge(projects::router())
            .merge(messages::router())
            .merge(invites::router())
            .merge(domains::router())
            .merge(api_users::router())
            .merge(whoami::router())
            .merge(subscriptions::router())
            .merge(api_keys::router())
            .merge(smtp_credentials::router())
            .merge(auth::router())
            .routes(routes!(crate::api::config))
            .routes(routes!(crate::api::healthy))
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
                get(verify_domain),
            )
            .fallback(api_fallback),
    );

    let api_doc = router.get_openapi_mut();
    #[cfg(not(feature = "internal-api-docs"))]
    hide_internal(api_doc);
    api_doc.info.title = "Remails API".to_string();
    api_doc.info.version = version.to_string();
    api_doc.info.contact = Some(
        ContactBuilder::new()
            .email(Some("info@remails.com"))
            .build(),
    );
    api_doc.info.description = Some(include_str!("openapi-description.md").to_string());

    router
}

pub fn docs_router() -> Router {
    let openapi = openapi_router().to_openapi();

    MemoryServe::new(load_assets!("src/static"))
        .index_file(Some("/scalar.html"))
        .fallback(Some("/scalar.html"))
        .into_router()
        .merge(
            Router::new()
                .route("/openapi.json", get(async move || Json(openapi)))
                .layer(CompressionLayer::new().br(true)),
        )
        .layer(RequestBodyLimitLayer::new(12_000))
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
}

async fn serve_docs(socket: SocketAddr, shutdown: CancellationToken) -> Result<(), ApiServerError> {
    let listener = TcpListener::bind(socket)
        .await
        .map_err(ApiServerError::Bind)?;

    info!("Docs server listening on {}", socket);

    axum::serve(
        listener,
        docs_router().into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(wait_for_shutdown(shutdown))
    .await
    .map_err(ApiServerError::Serve)
}

pub fn spawn_docs(socket: SocketAddr, shutdown: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = serve_docs(socket, shutdown.clone()).await {
            error!("server error: {:?}", e);
            shutdown.cancel();
            error!("shutting down API server")
        }
    })
}

#[cfg(not(feature = "internal-api-docs"))]
fn hide_internal(openapi: &mut utoipa::openapi::OpenApi) {
    openapi.paths.paths.iter_mut().for_each(|(_, item)| {
        if contains_internal_tag(&item.get) {
            item.get = None;
        }
        if contains_internal_tag(&item.put) {
            item.put = None;
        }
        if contains_internal_tag(&item.post) {
            item.post = None;
        }
        if contains_internal_tag(&item.delete) {
            item.delete = None;
        }
        if contains_internal_tag(&item.options) {
            item.options = None;
        }
        if contains_internal_tag(&item.head) {
            item.head = None;
        }
        if contains_internal_tag(&item.patch) {
            item.patch = None;
        }
        if contains_internal_tag(&item.trace) {
            item.trace = None;
        }
    });
}

#[cfg(not(feature = "internal-api-docs"))]
fn contains_internal_tag(operation: &Option<utoipa::openapi::path::Operation>) -> bool {
    if let Some(get) = operation
        && let Some(tags) = &get.tags
    {
        return tags.contains(&"internal".to_string());
    }
    false
}
