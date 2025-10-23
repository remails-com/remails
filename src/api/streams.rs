use crate::{
    api::{
        auth::Authenticated,
        error::{ApiError, ApiResult},
    },
    models::{NewStream, OrganizationId, ProjectId, Stream, StreamId, StreamRepository},
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use http::StatusCode;
use tracing::{debug, info};

pub async fn list_streams(
    State(repo): State<StreamRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id)): Path<(OrganizationId, ProjectId)>,
) -> ApiResult<Vec<Stream>> {
    user.has_org_read_access(&org_id)?;

    let streams = repo.list(org_id, proj_id).await?;

    debug!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        "listed {} streams",
        streams.len()
    );

    Ok(Json(streams))
}

pub async fn create_stream(
    State(repo): State<StreamRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id)): Path<(OrganizationId, ProjectId)>,
    Json(new): Json<NewStream>,
) -> Result<impl IntoResponse, ApiError> {
    user.has_org_write_access(&org_id)?;

    let stream = repo.create(new, org_id, proj_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream.id().to_string(),
        stream_name = stream.name,
        "created stream"
    );

    Ok((StatusCode::CREATED, Json(stream)))
}

pub async fn update_stream(
    State(repo): State<StreamRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
    Json(update): Json<NewStream>,
) -> ApiResult<Stream> {
    user.has_org_write_access(&org_id)?;

    let stream = repo.update(org_id, proj_id, stream_id, update).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream.id().to_string(),
        stream_name = stream.name,
        "updated stream"
    );

    Ok(Json(stream))
}

pub async fn remove_stream(
    State(repo): State<StreamRepository>,
    user: Box<dyn Authenticated>,
    Path((org_id, proj_id, stream_id)): Path<(OrganizationId, ProjectId, StreamId)>,
) -> ApiResult<StreamId> {
    user.has_org_write_access(&org_id)?;

    let stream_id = repo.remove(org_id, proj_id, stream_id).await?;

    info!(
        user_id = user.log_id(),
        organization_id = org_id.to_string(),
        project_id = proj_id.to_string(),
        stream_id = stream_id.to_string(),
        "deleted stream",
    );

    Ok(Json(stream_id))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::api::tests::{TestServer, deserialize_body, serialize_body};

    use super::*;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects")
    ))]
    async fn test_stream_lifecycle(pool: PgPool) {
        let user_a = "9244a050-7d72-451a-9248-4b43d5108235".parse().unwrap(); // is admin of org 1 and 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let server = TestServer::new(pool.clone(), Some(user_a)).await;

        // start without streams
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let streams: Vec<Stream> = deserialize_body(response.into_body()).await;
        assert_eq!(streams.len(), 0);

        // create a new stream
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams"),
                serialize_body(NewStream {
                    name: "Test Stream".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created_stream: Stream = deserialize_body(response.into_body()).await;
        assert_eq!(created_stream.name, "Test Stream");

        // list streams
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let streams: Vec<Stream> = deserialize_body(response.into_body()).await;
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "Test Stream");
        assert_eq!(streams[0].id(), created_stream.id());

        // update stream
        let response = server
            .put(
                format!(
                    "/api/organizations/{org_1}/projects/{proj_1}/streams/{}",
                    created_stream.id()
                ),
                serialize_body(NewStream {
                    name: "Updated Stream".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let stream: Stream = deserialize_body(response.into_body()).await;
        assert_eq!(stream.name, "Updated Stream");
        assert_eq!(stream.id(), created_stream.id());

        // list streams
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let streams: Vec<Stream> = deserialize_body(response.into_body()).await;
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "Updated Stream");
        assert_eq!(streams[0].id(), created_stream.id());

        // remove stream
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{}",
                created_stream.id()
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // verify stream is removed
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let streams: Vec<Stream> = deserialize_body(response.into_body()).await;
        assert_eq!(streams.len(), 0);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "api_users", "projects", "streams")
    ))]
    async fn test_stream_no_access(pool: PgPool) {
        let user_b = "94a98d6f-1ec0-49d2-a951-92dc0ff3042a".parse().unwrap(); // is admin of org 2
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176";
        let proj_1 = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462"; // project 1 in org 1
        let stream_1 = "85785f4c-9167-4393-bbf2-3c3e21067e4a"; // stream 1 in project 1
        let server = TestServer::new(pool.clone(), Some(user_b)).await;

        // can't list streams
        let response = server
            .get(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't create streams
        let response = server
            .post(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams"),
                serialize_body(NewStream {
                    name: "Test Stream".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't update stream
        let response = server
            .put(
                format!("/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}"),
                serialize_body(NewStream {
                    name: "Updated Stream".to_string(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // can't delete stream
        let response = server
            .delete(format!(
                "/api/organizations/{org_1}/projects/{proj_1}/streams/{stream_1}"
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
