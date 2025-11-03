use crate::{
    api::{
        ApiState, RemailsConfig,
        auth::Authenticated,
        error::{ApiError, ApiResult},
        validation::ValidatedJson,
    },
    handler::dns::{DnsResolver, DomainVerificationStatus},
    models::{ApiDomain, DomainId, DomainRepository, NewDomain, OrganizationId, ProjectId},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use serde::Deserialize;
use tracing::{debug, info};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(create_domain, list_domains))
        .routes(routes!(get_domain, delete_domain))
        .routes(routes!(verify_domain))
}

#[derive(Debug, Deserialize)]
pub struct DomainPath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
}

#[derive(Debug, Deserialize)]
pub struct SpecificDomainPath {
    org_id: OrganizationId,
    project_id: Option<ProjectId>,
    domain_id: DomainId,
}

/// Create a new domain
///
/// TODO: utoipa does not support registering the same handler with to different paths yet.
///  Looking at #344, I did not spend effort making this possible.
#[utoipa::path(post, path = "/organizations/{org_id}/domains",
    tags = ["Domains"],
    params(OrganizationId),
    request_body = NewDomain,
    responses(
        (status = 201, description = "Domain successfully created", body = ApiDomain),
        ApiError,
    )
)]
pub(crate) async fn create_domain(
    State(repo): State<DomainRepository>,
    State(resolver): State<DnsResolver>,
    State(config): State<RemailsConfig>,
    user: Box<dyn Authenticated>,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
    ValidatedJson(new): ValidatedJson<NewDomain>,
) -> Result<impl IntoResponse, ApiError> {
    user.has_org_write_access(&org_id)?;

    let domain = repo.create(new, org_id, project_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    let domain = ApiDomain::verified(domain, status);

    info!(
        user_id = user.log_id(),
        domain_id = domain.id().to_string(),
        parent_id = ?domain.parent_id(),
        domain = domain.domain(),
        "created domain");

    Ok((StatusCode::CREATED, Json(domain)))
}

/// List all domains
///
/// TODO: utoipa does not support registering the same handler with to different paths yet.
///  Looking at #344, I did not spend effort making this possible.
#[utoipa::path(get, path = "/organizations/{org_id}/domains",
    tags = ["Domains"],
    params(OrganizationId),
    responses(
        (status = 200, description = "Successfully fetched domains", body = [ApiDomain]),
        ApiError,
    )
)]
pub(crate) async fn list_domains(
    State(repo): State<DomainRepository>,
    State(resolver): State<DnsResolver>,
    State(config): State<RemailsConfig>,
    user: Box<dyn Authenticated>,
    Path(DomainPath { org_id, project_id }): Path<DomainPath>,
) -> ApiResult<Vec<ApiDomain>> {
    user.has_org_read_access(&org_id)?;

    let domains = repo.list(org_id, project_id).await?;

    let mut verified_domains = Vec::with_capacity(domains.len());

    for domain in domains {
        let status = resolver.verify_domain(&domain, &config.spf_include).await?;

        verified_domains.push(ApiDomain::verified(domain, status));
    }

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        "verified & listed {} domains",
        verified_domains.len()
    );

    Ok(Json(verified_domains))
}

/// Get domain by ID
///
/// TODO: utoipa does not support registering the same handler with to different paths yet.
///  Looking at #344, I did not spend effort making this possible.
#[utoipa::path(get, path = "/organizations/{org_id}/domains/{domain_id}",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully fetched domain", body = ApiDomain),
        ApiError,
    )
)]
pub async fn get_domain(
    State(repo): State<DomainRepository>,
    State(resolver): State<DnsResolver>,
    State(config): State<RemailsConfig>,
    user: Box<dyn Authenticated>,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<ApiDomain> {
    user.has_org_read_access(&org_id)?;

    let domain = repo.get(org_id, project_id, domain_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    let domain = ApiDomain::verified(domain, status);

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        domain_id = domain_id.to_string(),
        domain = domain.domain(),
        "retrieved domain",
    );

    Ok(Json(domain))
}

/// Delete domain
///
/// TODO: utoipa does not support registering the same handler with to different paths yet.
///  Looking at #344, I did not spend effort making this possible.
#[utoipa::path(delete, path = "/organizations/{org_id}/domains/{domain_id}",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully deleted domain", body = DomainId),
        ApiError,
    )
)]
pub async fn delete_domain(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<DomainId> {
    user.has_org_write_access(&org_id)?;

    let domain_id = repo.remove(org_id, project_id, domain_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = project_id.map(|id| id.to_string()),
        domain_id = domain_id.to_string(),
        "deleted domain",
    );

    Ok(Json(domain_id))
}

/// Verify domain
///
/// Checks if the various DNS entries required or recommended for sending are set correctly.
///
/// TODO: utoipa does not support registering the same handler with to different paths yet.
///  Looking at #344, I did not spend effort making this possible.
#[utoipa::path(post, path = "/organizations/{org_id}/domains/{domain_id}/verify",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully verified domain", body = DomainVerificationStatus),
        ApiError,
    )
)]
pub(super) async fn verify_domain(
    State(repo): State<DomainRepository>,
    State(resolver): State<DnsResolver>,
    State(config): State<RemailsConfig>,
    user: Box<dyn Authenticated>,
    Path(SpecificDomainPath {
        org_id,
        project_id,
        domain_id,
    }): Path<SpecificDomainPath>,
) -> ApiResult<DomainVerificationStatus> {
    user.has_org_read_access(&org_id)?;

    let domain = repo.get(org_id, project_id, domain_id).await?;

    let status = resolver.verify_domain(&domain, &config.spf_include).await?;

    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::DkimKeyType,
    };

    use super::*;

    async fn test_domain_lifecycle(pool: PgPool, endpoint: String) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let server = TestServer::new(pool.clone(), Some(user_a)).await;

        // start without domains
        let response = server.get(format!("{endpoint}/domains")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domains: Vec<ApiDomain> = deserialize_body(response.into_body()).await;
        assert_eq!(domains.len(), 0);

        // create a new domain
        let response = server
            .post(
                format!("{endpoint}/domains"),
                serialize_body(NewDomain {
                    domain: "remails.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_domain: ApiDomain = deserialize_body(response.into_body()).await;
        assert_eq!(created_domain.domain(), "remails.com");

        // list domains
        let response = server.get(format!("{endpoint}/domains")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domains: Vec<ApiDomain> = deserialize_body(response.into_body()).await;
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].domain(), "remails.com");
        assert_eq!(domains[0].id(), created_domain.id());

        // get domain
        let response = server
            .get(format!("{endpoint}/domains/{}", created_domain.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domain: ApiDomain = deserialize_body(response.into_body()).await;
        assert_eq!(domain.domain(), "remails.com");
        assert_eq!(domain.id(), created_domain.id());

        // verify domain
        let response = server
            .post(
                format!("{endpoint}/domains/{}/verify", created_domain.id()),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _: DomainVerificationStatus = deserialize_body(response.into_body()).await;

        // remove domain
        let response = server
            .delete(format!("{endpoint}/domains/{}", created_domain.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify domain is removed
        let response = server.get(format!("{endpoint}/domains")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domains: Vec<ApiDomain> = deserialize_body(response.into_body()).await;
        assert_eq!(domains.len(), 0);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn test_project_domains_lifecycle(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let endpoint = format!("/api/organizations/{org_1}/projects/{proj_1}");
        test_domain_lifecycle(pool, endpoint).await;
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_org_domains_lifecycle(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let endpoint = format!("/api/organizations/{org_1}");
        test_domain_lifecycle(pool, endpoint).await;
    }

    async fn test_domains_no_access(pool: PgPool, endpoint: String, domain_id: &str) {
        let user_b = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is only member of org 2
        let server = TestServer::new(pool.clone(), Some(user_b)).await;

        // can't list domains for other organizations
        let response = server.get(format!("{endpoint}/domains")).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't create domain for other organizations
        let response = server
            .post(
                format!("{endpoint}/domains"),
                serialize_body(NewDomain {
                    domain: "remails.com".to_string(),
                    dkim_key_type: DkimKeyType::RsaSha256,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't get domain for other organizations
        let response = server
            .get(format!("{endpoint}/domains/{domain_id}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't delete domain for other organizations
        let response = server
            .delete(format!("{endpoint}/domains/{domain_id}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't verify domain for other organizations
        let response = server
            .post(
                format!("{endpoint}/domains/{domain_id}/verify"),
                Body::empty(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "proj_domains")
    ))]
    async fn test_project_domains_no_access(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let endpoint = format!("/api/organizations/{org_1}/projects/{proj_1}");
        let proj_domain = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a"; // test-org-1-project-1.com
        test_domains_no_access(pool, endpoint, proj_domain).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "org_domains")
    ))]
    async fn test_org_domains_no_access(pool: PgPool) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let endpoint = format!("/api/organizations/{org_1}");
        let org_domain = "ed28baa5-57f7-413f-8c77-7797ba6a8780"; // test-org-1.com
        test_domains_no_access(pool, endpoint, org_domain).await;
    }
}
