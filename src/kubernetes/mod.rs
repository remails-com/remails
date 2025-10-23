mod mock_k8s_api;
mod node_watcher;

use crate::kubernetes::mock_k8s_api::mock_service;
use k8s_openapi::api::core::v1::Node;
use kube::Api;
use sqlx::{PgPool, types::ipnet::IpNet};
use std::env;
use tracing::{error, info};

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
            kube::Client::new(mock_service(), "default")
        } else {
            kube::Client::try_default().await.unwrap_or_else(|e| {
                error!("Cloud not connect to real Kubernetes API, using Mock API instead: {e}");
                kube::Client::new(mock_service(), "default")
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
