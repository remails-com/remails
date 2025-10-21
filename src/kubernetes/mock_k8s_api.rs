use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use k8s_openapi::{
    List,
    api::core::v1::{Node, NodeCondition, NodeSpec, NodeStatus},
    apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta},
};
use std::{
    collections::BTreeSet,
    fmt::Display,
    ops::Deref,
    sync::{Arc, RwLock},
};
use tracing::info;

#[derive(Clone)]
struct ApiState {
    node_names: Arc<RwLock<BTreeSet<String>>>,
}

fn nodes<T, D>(names: T) -> Vec<Node>
where
    T: IntoIterator<Item = D>,
    D: Display,
{
    names.into_iter().map(|name| node(name)).collect()
}

fn node<D: Display>(name: D) -> Node {
    Node {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        spec: Some(NodeSpec {
            provider_id: Some(format!("k8s:////{name}-provider-id")),
            ..Default::default()
        }),
        status: Some(NodeStatus {
            conditions: Some(vec![NodeCondition {
                status: "True".to_string(),
                type_: "Ready".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        }),
    }
}

/// GET /api/v1/nodes
async fn list_nodes(State(state): State<ApiState>) -> impl IntoResponse {
    info!("called `list_nodes`");
    let node_names = state.node_names.read().unwrap();
    // The K8s API returns a NodeList object, not just an array of Nodes
    let node_list: List<Node> = List {
        items: nodes(node_names.deref()).to_vec(),
        metadata: ListMeta {
            // Important field for clients, mock it
            resource_version: Some("999".to_string()),
            ..Default::default()
        },
    };

    info!(
        "Returning {} nodes ({:?})",
        node_list.items.len(),
        node_list.items
    );

    (StatusCode::OK, Json(node_list))
}

/// GET /api/v1/nodes/{name}
async fn get_node(Path(name): Path<String>, State(state): State<ApiState>) -> impl IntoResponse {
    info!("called `get_node` with name: {name}");
    let mut node_names = state.node_names.write().unwrap();

    match name {
        s if node_names.contains(&s) => (StatusCode::OK, Json(node(s))).into_response(),
        s => {
            node_names.insert(s.clone());
            (StatusCode::OK, Json(node(s))).into_response()
        }
    }
}

pub(super) fn mock_service() -> Router {
    Router::new()
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes/{name}", get(get_node))
        .with_state(ApiState {
            node_names: Arc::new(RwLock::new(BTreeSet::new())),
        })
}
