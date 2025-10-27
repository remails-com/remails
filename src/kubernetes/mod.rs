mod mock_k8s_api;

use crate::kubernetes::mock_k8s_api::mock_service;
use k8s_openapi::api::core::v1::Node;
use kube::{Api, api::ListParams};
use sqlx::{PgPool, types::ipnet::IpNet};
use std::env;
use tracing::{error, info, trace, warn};

#[derive(Clone)]
pub struct Kubernetes {
    client: kube::Client,
    db: PgPool,
    node_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes API error: {0:?}")]
    Api(#[from] kube::Error),
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Resource not found")]
    NotFound,
}

impl Kubernetes {
    pub async fn new(db: PgPool) -> Result<Self, Error> {
        let client = if env::var("K8S_API_MOCK")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            info!("Requested to use mock K8s API");
            kube::Client::new(mock_service().0, "default")
        } else {
            kube::Client::try_default().await.unwrap_or_else(|e| {
                error!("Cloud not connect to real Kubernetes API, using Mock API instead: {e}");
                kube::Client::new(mock_service().0, "default")
            })
        };

        let node_name =
            env::var("K8S_NODE_HOSTNAME").expect("Missing K8S_NODE_HOSTNAME environment variable");

        Ok(Self {
            client,
            db,
            node_name,
        })
    }

    /// Queries the Kubernetes API server for all nodes and their health status.
    ///
    /// ## Returns
    /// A tuple containing three vectors:
    /// 1. A vector of node hostnames.
    /// 2. A vector of provider IDs.
    /// 3. A vector of booleans indicating whether each node is ready
    ///
    /// The reason to make this a tuple of vectors is that it allows easier interaction with SQLx
    async fn list_nodes_health_from_api_server(
        &self,
    ) -> Result<(Vec<String>, Vec<String>, Vec<bool>), Error> {
        let node_api: Api<Node> = Api::all(self.client.clone());
        let mut hostnames = Vec::new();
        let mut provider_ids = Vec::new();
        let mut ready = Vec::new();

        let mut list_params = ListParams::default();
        loop {
            let list_res = node_api.list(&ListParams::default()).await?;

            for node in list_res.items {
                let Some(node_name) = node.metadata.name else {
                    error!("Kubernetes API server node does not have a name");
                    continue;
                };

                let provider_id = match node
                    .spec
                    .as_ref()
                    .and_then(|spec| spec.provider_id.as_ref())
                {
                    Some(pid) => pid.clone(),
                    None => {
                        error!("Kubernetes node {} does not have a provider ID", node_name);
                        continue;
                    }
                };

                let is_ready = node
                    .status
                    .and_then(|status| {
                        status.conditions.and_then(|conditions| {
                            conditions.into_iter().find_map(|condition| {
                                if condition.type_ == "Ready" {
                                    Some(condition.status == "True")
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .unwrap_or(false);

                hostnames.push(node_name);
                provider_ids.push(provider_id);
                ready.push(is_ready);
            }

            match list_res.metadata.continue_ {
                None => break,
                Some(token) => list_params.continue_token = Some(token),
            }
        }
        debug_assert!(hostnames.len() == ready.len());
        debug_assert!(hostnames.len() == provider_ids.len());

        Ok((hostnames, provider_ids, ready))
    }

    pub async fn check_node_health(&self) -> Result<(), Error> {
        let (hostnames, provider_ids, ready) = self.list_nodes_health_from_api_server().await?;

        // Remove all nodes that are not in the API server anymore
        let removed_hostnames = sqlx::query_scalar!(
            r#"
            DELETE FROM k8s_nodes WHERE NOT hostname = ANY($1)
            RETURNING hostname
            "#,
            &hostnames
        )
        .fetch_all(&self.db)
        .await?;

        for hostname in &removed_hostnames {
            info!(
                hostname,
                "Kubernetes node has been removed from the cluster"
            );
        }

        let nodes = sqlx::query!(
            r#"
            INSERT INTO k8s_nodes (id, hostname, provider_id, ready)
            VALUES (gen_random_uuid(), unnest($1::text[]), unnest($2::text[]), unnest($3::bool[]))
            ON CONFLICT (hostname) DO UPDATE
                SET ready = EXCLUDED.ready
            RETURNING hostname, ready
            "#,
            &hostnames,
            &provider_ids,
            &ready
        )
        .fetch_all(&self.db)
        .await?;

        for node in nodes {
            if node.ready {
                trace!(node.hostname, "Kubernetes node is ready");
            } else {
                warn!(node.hostname, "Kubernetes node is NOT ready");
            }
        }

        Ok(())
    }

    async fn get_provider_id(&self, node_name: &str) -> Result<String, Error> {
        let nodes: Api<Node> = Api::all(self.client.clone());
        let node = nodes.get(node_name).await?;
        node.spec
            .ok_or(Error::NotFound)?
            .provider_id
            .ok_or(Error::NotFound)
    }

    pub async fn save_available_node_ips<T>(&self, ips: T) -> Result<(), Error>
    where
        T: IntoIterator,
        T::Item: Into<IpNet>,
    {
        let mut tx = self.db.begin().await?;

        let node_id = match sqlx::query_scalar!(
            r#"
            SELECT id FROM k8s_nodes WHERE hostname = $1
            "#,
            self.node_name
        )
        .fetch_optional(&mut *tx)
        .await?
        {
            Some(id) => id,
            None => {
                info!(
                    node_name = self.node_name,
                    "Adding new kubernetes node to database"
                );
                let provider_id = self.get_provider_id(&self.node_name).await?;

                sqlx::query_scalar!(
                    r#"
                    INSERT INTO k8s_nodes (id, provider_id, hostname)
                    VALUES (uuid_generate_v4(), $1, $2)
                    RETURNING id
                    "#,
                    provider_id,
                    self.node_name
                )
                .fetch_one(&mut *tx)
                .await?
            }
        };

        let ips = ips.into_iter().map(Into::into).collect::<Vec<IpNet>>();

        // De-assign IPs that are no longer available
        // They should only be deleted if we are actually not in control over them anymore, e.g.,
        // if we deleted the FloatingIP in UpCloud
        sqlx::query!(
            r#"
            UPDATE outbound_ips
            SET node_id = NULL
            WHERE node_id = $1
              AND ip = ANY($2::inet[])
            "#,
            node_id,
            &ips
        )
        .execute(&mut *tx)
        .await?;

        // Insert new IPs
        sqlx::query!(
            r#"
            INSERT INTO outbound_ips (id, node_id, ip)
            VALUES (uuid_generate_v4(), $1, unnest($2::inet[]))
            ON CONFLICT (ip) DO UPDATE
                SET node_id = $1
            "#,
            node_id,
            &ips,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Kubernetes {
        async fn with_kube_client(pool: PgPool, client: kube::Client) -> Result<Self, Error> {
            Ok(Self {
                client,
                db: pool,
                node_name: "mock-node-1".to_string(),
            })
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("k8s_nodes")))]
    async fn test_list_nodes_health_from_api_server(pool: PgPool) {
        let current_node_count = sqlx::query_scalar!(
            r#"
            SELECT count(*) FROM k8s_nodes
            "#
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();

        assert_eq!(current_node_count, 2);

        let (mock_router, mock_state) = mock_service();
        let kube_client = kube::Client::new(mock_router, "default");
        let k8s = Kubernetes::with_kube_client(pool.clone(), kube_client)
            .await
            .unwrap();

        mock_state.add_node("mock-node-1");
        k8s.check_node_health().await.unwrap();

        let nodes = sqlx::query!(
            r#"
            SELECT hostname, ready FROM k8s_nodes
            "#
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].hostname, "mock-node-1");
        assert!(nodes[0].ready);

        mock_state.add_node("mock-node-2");
        mock_state.set_ready("mock-node-1", false);
        k8s.check_node_health().await.unwrap();

        let nodes = sqlx::query!(
            r#"
            SELECT hostname, ready FROM k8s_nodes
            "#
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(nodes.len(), 2);
        // mock-node-2 should be ready, mock-node-1 should not be ready.
        // The order of them in the vec is not guaranteed, so we just check that they are different.
        assert_ne!(nodes[0].ready, nodes[1].ready);
    }
}
