use crate::api::{ApiState, error, messages, organizations, projects, whoami};
use utoipa::{
    OpenApi,
    openapi::{
        ContactBuilder, SecurityRequirement,
        path::Operation,
        security::{Http, HttpAuthScheme, SecurityScheme},
    },
};
use utoipa_axum::router::OpenApiRouter;

pub fn openapi_router() -> OpenApiRouter<ApiState> {
    #[derive(utoipa::OpenApi)]
    #[openapi(components(schemas(error::ApiErrorResponse, crate::models::StreamId)))]
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
            .merge(whoami::router()),
    );

    let api_doc = router.get_openapi_mut();
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

fn contains_internal_tag(operation: &Option<Operation>) -> bool {
    if let Some(get) = operation
        && let Some(tags) = &get.tags
    {
        return tags.contains(&"internal".to_string());
    }
    false
}
