use crate::api::{
    ApiState, api_fallback, api_keys, api_users,
    auth::{logout, password_login, password_register, totp_login},
    domains,
    domains::{create_domain, delete_domain, get_domain, list_domains, verify_domain},
    error, invites, messages, organizations, projects, smtp_credentials, streams, subscriptions,
    whoami,
};
use axum::routing::{get, post};
use utoipa::{
    OpenApi,
    openapi::{
        ContactBuilder, SecurityRequirement,
        security::{Http, HttpAuthScheme, SecurityScheme},
    },
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn openapi_router() -> OpenApiRouter<ApiState> {
    #[derive(utoipa::OpenApi)]
    #[openapi(components(schemas(
        error::ApiErrorResponse,
        crate::models::StreamId,
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
    let mut components = api_doc.components.unwrap_or_default();
    components.add_security_scheme("basicAuth", security);
    api_doc.components = Some(components);
    let mut security = api_doc.security.unwrap_or_default();
    security.push(SecurityRequirement::new::<&str, [_; 0], String>(
        "basicAuth",
        [],
    ));
    api_doc.security = Some(security);

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
            .merge(streams::router())
            .merge(smtp_credentials::router())
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
                post(verify_domain),
            )
            .route("/logout", get(logout))
            .route("/login/password", post(password_login))
            .route("/login/totp", post(totp_login))
            .route("/register/password", post(password_register))
            .fallback(api_fallback),
    );

    let api_doc = router.get_openapi_mut();
    #[cfg(not(feature = "internal-api-docs"))]
    hide_internal(api_doc);
    api_doc.info.title = "Remails API".to_string();
    api_doc.info.contact = Some(
        ContactBuilder::new()
            .email(Some("info@remails.com"))
            .build(),
    );
    // info.description = Some(
    //     "Remails is a transactional email service focused on privacy and deliverability."
    //         .to_string(),
    // );

    router
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
