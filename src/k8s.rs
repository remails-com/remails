use crate::Environment;
use k8s_openapi::api::core::v1::Node;
use kube::Api;
use sqlx::{PgPool, types::ipnet::IpNet};
use std::env;
use tracing::{info, warn};

#[derive(Clone)]
pub struct Kubernetes {
    client: kube::Client,
    db: PgPool,
    node_name: String,
    environment: Environment,
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
        let client = kube::Client::try_default().await?;
        let node_name =
            env::var("K8S_NODE_HOSTNAME").expect("Missing K8S_NODE_HOSTNAME environment variable");
        let environment: Environment = env::var("ENVIRONMENT")
            .map(|s| s.parse())
            .inspect_err(|_| warn!("Did not find ENVIRONMENT env var, defaulting to development"))
            .unwrap_or(Ok(Environment::Development))
            .expect(
                "Invalid ENVIRONMENT env var, must be one of: development, production, or staging",
            );

        Ok(Self {
            client,
            db,
            node_name,
            environment,
        })
    }

    async fn get_provider_id(&self, node_name: &str) -> Result<String, Error> {
        if matches!(self.environment, Environment::Development) {
            // TODO: tink of a better way for non-k8s environments (including a better detection)
            return Ok("k8s:////development-node".to_string());
        }

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

        // Delete IPs that are no longer available
        sqlx::query!(
            r#"
            DELETE FROM outbound_ips WHERE node_id = $1 AND ip = ANY($2::inet[])
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
