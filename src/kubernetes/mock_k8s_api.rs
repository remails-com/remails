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
    collections::BTreeMap,
    fmt::Display,
    sync::{Arc, RwLock},
};
use tracing::info;

#[derive(Clone)]
pub struct ApiState {
    nodes: Arc<RwLock<BTreeMap<String, Node>>>,
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
    let nodes = state.nodes.read().unwrap();
    // The K8s API returns a NodeList object, not just an array of Nodes
    let node_list: List<Node> = List {
        items: nodes.values().cloned().collect(),
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
    let mut nodes = state.nodes.write().unwrap();

    let node = nodes.entry(name.clone()).or_insert(node(&name));

    (StatusCode::OK, Json(node)).into_response()
}

pub(super) fn mock_service() -> (Router, ApiState) {
    let state = ApiState {
        nodes: Arc::new(RwLock::new(BTreeMap::new())),
    };
    let router = Router::new()
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes/{name}", get(get_node))
        .with_state(state.clone());

    (router, state)
}

#[cfg(test)]
mod tests {
    use crate::kubernetes::mock_k8s_api::{ApiState, node};
    use k8s_openapi::api::core::v1::NodeCondition;
    use std::fmt::Display;

    impl ApiState {
        pub fn add_node<D: Display>(&self, name: D) {
            let mut nodes = self.nodes.write().unwrap();
            nodes.insert(name.to_string(), node(name));
        }

        pub fn set_ready(&self, node_name: &str, ready: bool) {
            let mut nodes = self.nodes.write().unwrap();
            if let Some(node) = nodes.get_mut(node_name)
                && let Some(status) = &mut node.status
            {
                status.conditions = Some(vec![NodeCondition {
                    status: if ready {
                        "True".to_string()
                    } else {
                        "False".to_string()
                    },
                    type_: "Ready".to_string(),
                    ..Default::default()
                }]);
            }
        }
    }
}
