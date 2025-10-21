// Inspired by https://github.com/kube-rs/kube/blob/dac48d9/examples/node_watcher.rs

use std::pin::pin;

use crate::kubernetes::Error;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::Api,
    client::Client,
    runtime::{WatchStreamExt, watcher},
};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::*;

pub fn spawn_node_watcher(
    db: PgPool,
    client: Client,
    shutdown: CancellationToken,
) -> Result<JoinHandle<()>, Error> {
    let nodes: Api<Node> = Api::all(client.clone());

    let wc = watcher::Config::default();
    let obs = watcher(nodes, wc).default_backoff().applied_objects();

    Ok(tokio::spawn(async move {
        let mut obs = pin!(obs);
        loop {
            match obs.try_next().await {
                Ok(Some(n)) => {
                    check_for_node_failures(n, &db)
                        .await
                        .inspect_err(|e| error!("error checking node: {}", e))
                        .ok();
                }
                Ok(None) => {
                    error!("K8s API watcher stream ended");
                    shutdown.cancel();
                    break;
                }
                Err(e) => {
                    error!("K8s API watcher stream encountered an error: {e:?}");
                    shutdown.cancel();
                    break;
                }
            }
        }
    }))
}

async fn check_for_node_failures(node: Node, db: &PgPool) -> Result<(), Error> {
    let Some(node_name) = node.metadata.name else {
        error!("node has no name");
        return Ok(());
    };

    let provider_id = if let Some(spec) = node.spec
        && let Some(provider_id) = spec.provider_id
    {
        provider_id
    } else {
        error!("node has no provider_id");
        return Ok(());
    };

    // Nodes often modify a lot - only print broken nodes
    let status = node
        .status
        .unwrap()
        .conditions
        .unwrap()
        .into_iter()
        .find(|c| c.type_ == "Ready")
        .map(|c| c.status == "True");

    let is_up = matches!(status, Some(true));

    if is_up {
        info!(node_name = node_name, "node up");
    } else {
        warn!(node_name = node_name, "node down");
    }

    sqlx::query!(
        r#"
        INSERT INTO k8s_nodes (id, hostname, provider_id, ready) VALUES (uuid_generate_v4(), $1, $2, $3)
        ON CONFLICT (hostname) DO UPDATE SET ready = $3
        "#,
        node_name,
        provider_id,
        is_up,
    ).execute(db).await?;

    Ok(())
}
