use crate::{
    api::{
        ApiState,
        auth::Authenticated,
        error::{ApiResult, AppError},
        validation::ValidatedJson,
    },
    handler::dns::DomainVerificationStatus,
    models::{ApiDomain, DomainId, DomainRepository, NewDomain, OrganizationId, ProjectId},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(create_domain, list_domains))
        .routes(routes!(get_domain, delete_domain, update_domain))
        .routes(routes!(verify_domain))
}

/// Create a new domain
#[utoipa::path(post, path = "/organizations/{org_id}/domains",
    tags = ["Domains"],
    params(OrganizationId),
    request_body = NewDomain,
    responses(
        (status = 201, description = "Domain successfully created", body = ApiDomain),
        AppError,
    )
)]
pub(crate) async fn create_domain(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path(org_id): Path<OrganizationId>,
    ValidatedJson(new): ValidatedJson<NewDomain>,
) -> Result<impl IntoResponse, AppError> {
    user.has_org_write_access(&org_id)?;

    let domain: ApiDomain = repo.create(new, org_id).await?.into();

    info!(
        user_id = user.log_id(),
        domain_id = domain.id().to_string(),
        organization_id = ?domain.organization_id(),
        domain = domain.domain(),
        "created domain");

    Ok((StatusCode::CREATED, Json(domain)))
}

/// List all domains
#[utoipa::path(get, path = "/organizations/{org_id}/domains",
    tags = ["Domains"],
    params(OrganizationId),
    responses(
        (status = 200, description = "Successfully fetched domains", body = [ApiDomain]),
        AppError,
    )
)]
pub(crate) async fn list_domains(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path(org_id): Path<OrganizationId>,
) -> ApiResult<Vec<ApiDomain>> {
    user.has_org_read_access(&org_id)?;

    let domains: Vec<ApiDomain> = repo
        .list(org_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        "listed {} domains",
        domains.len()
    );

    Ok(Json(domains))
}

/// Get domain by ID
#[utoipa::path(get, path = "/organizations/{org_id}/domains/{domain_id}",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully fetched domain", body = ApiDomain),
        AppError,
    )
)]
pub async fn get_domain(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, domain_id)): Path<(OrganizationId, DomainId)>,
) -> ApiResult<ApiDomain> {
    user.has_org_read_access(&org_id)?;

    let domain: ApiDomain = repo.get(org_id, domain_id).await?.into();

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        domain_id = domain_id.to_string(),
        domain = domain.domain(),
        "retrieved domain",
    );

    Ok(Json(domain))
}

/// Update domain
#[utoipa::path(put, path = "/organizations/{org_id}/domains/{domain_id}",
    tags = ["Domains"],
    request_body = Option<ProjectId>,
    responses(
        (status = 200, description = "Domain successfully updated", body = ApiDomain),
        AppError,
    )
)]
pub async fn update_domain(
    State(repo): State<DomainRepository>,
    Path((org_id, domain_id)): Path<(OrganizationId, DomainId)>,
    user: Box<dyn Authenticated>,
    Json(update): Json<Vec<ProjectId>>,
) -> ApiResult<ApiDomain> {
    user.has_org_write_access(&org_id)?;

    let domain = repo.update(org_id, domain_id, &update).await?.into();

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        domain_id = domain_id.to_string(),
        project_ids_len = update.len(),
        "updated domain",
    );

    Ok(Json(domain))
}

/// Delete domain
#[utoipa::path(delete, path = "/organizations/{org_id}/domains/{domain_id}",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully deleted domain", body = DomainId),
        AppError,
    )
)]
pub async fn delete_domain(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, domain_id)): Path<(OrganizationId, DomainId)>,
) -> ApiResult<DomainId> {
    user.has_org_write_access(&org_id)?;

    let domain_id = repo.remove(org_id, domain_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        domain_id = domain_id.to_string(),
        "deleted domain",
    );

    Ok(Json(domain_id))
}

/// Verify domain
///
/// Checks if the various DNS entries required or recommended for sending are set correctly.
#[utoipa::path(get, path = "/organizations/{org_id}/domains/{domain_id}/verify",
    tags = ["Domains"],
    params(OrganizationId, DomainId),
    responses(
        (status = 200, description = "Successfully verified domain", body = DomainVerificationStatus),
        AppError,
    )
)]
pub(super) async fn verify_domain(
    State(repo): State<DomainRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, domain_id)): Path<(OrganizationId, DomainId)>,
) -> ApiResult<DomainVerificationStatus> {
    user.has_org_read_access(&org_id)?;

    let status = repo.verify(org_id, domain_id).await?;

    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::{
        api::tests::{TestServer, deserialize_body, serialize_body},
        models::{DkimKeyType, ProjectId},
    };

    use super::*;

    async fn test_domain_lifecycle(pool: PgPool, project_ids: Vec<ProjectId>) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let endpoint = format!("/api/organizations/{org_1}");

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
                    project_ids: project_ids.clone(),
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
        assert_eq!(domains[0].id(), created_domain.id());
        assert_eq!(domains[0].domain(), "remails.com");
        assert_eq!(domains[0].organization_id(), org_1.parse().unwrap());
        assert_eq!(domains[0].project_ids(), project_ids);

        // update domain with new project ID
        let proj_2: ProjectId = "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap();
        let response = server
            .put(
                format!("{endpoint}/domains/{}", created_domain.id()),
                serialize_body(vec![proj_2]),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domain: ApiDomain = deserialize_body(response.into_body()).await;
        assert_eq!(domain.id(), created_domain.id());
        assert_eq!(domain.domain(), "remails.com");
        assert_eq!(domain.project_ids(), vec![proj_2]);

        // get domain
        let response = server
            .get(format!("{endpoint}/domains/{}", created_domain.id()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domain: ApiDomain = deserialize_body(response.into_body()).await;
        assert_eq!(domain.id(), created_domain.id());
        assert_eq!(domain.domain(), "remails.com");
        assert_eq!(domain.project_ids(), vec![proj_2]);

        // update domain with removed project ID
        let response = server
            .put(
                format!("{endpoint}/domains/{}", created_domain.id()),
                serialize_body(Vec::<DomainId>::new()),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let domain: ApiDomain = deserialize_body(response.into_body()).await;
        assert_eq!(domain.id(), created_domain.id());
        assert_eq!(domain.domain(), "remails.com");
        assert_eq!(domain.project_ids(), vec![]);

        // verify domain
        let response = server
            .get(format!("{endpoint}/domains/{}/verify", created_domain.id()))
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
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        test_domain_lifecycle(pool, vec![proj_1.parse().unwrap()]).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn test_org_domains_lifecycle(pool: PgPool) {
        test_domain_lifecycle(pool, vec![]).await;
    }

    async fn test_domains_no_access(pool: PgPool, domain_id: &str, project_ids: Vec<ProjectId>) {
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let endpoint = format!("/api/organizations/{org_1}");

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
                    project_ids,
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
            .get(format!("{endpoint}/domains/{domain_id}/verify"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "proj_domains")
    ))]
    async fn test_project_domains_no_access(pool: PgPool) {
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let proj_domain = "c1a4cc6c-a975-4921-a55c-5bfeb31fd25a"; // test-org-1-project-1.com
        test_domains_no_access(pool, proj_domain, vec![proj_1.parse().unwrap()]).await;
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "api_users", "org_domains")
    ))]
    async fn test_org_domains_no_access(pool: PgPool) {
        let org_domain = "ed28baa5-57f7-413f-8c77-7797ba6a8780"; // test-org-1.com
        test_domains_no_access(pool, org_domain, vec![]).await;
    }
}
