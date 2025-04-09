#![allow(dead_code)]
use derive_more::From;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Map;
use std::env;
use url::Url;

type Id = u64;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Serialize)]
struct JsonRpc {
    jsonrpc: String,
    method: String,
    id: Id,
    params: RpcParams,
}

#[derive(Serialize)]
struct RpcParams {
    service: String,
    method: String,
    args: Vec<serde_json::Value>,
}

#[derive(derive_more::Debug)]
struct Odoo {
    reqwest_client: reqwest::Client,
    base_url: Url,
    db_name: String,
    user_id: Id,
    #[debug("*****")]
    api_key: String,
}

#[non_exhaustive]
enum OdooMethod {
    SearchRead,
    Read,
}

impl OdooMethod {
    fn as_str(&self) -> &str {
        match self {
            OdooMethod::SearchRead => "search_read",
            OdooMethod::Read => "read",
        }
    }
}

struct RequestBuilder {
    resource: String,
    method: OdooMethod,
    args: Vec<serde_json::Value>,
    kwargs: Map<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum OdooResponse<T> {
    Success { result: T },
    Failure { error: serde_json::Value },
}

#[cfg(test)]
impl<T> OdooResponse<T> {
    fn is_success(&self) -> bool {
        matches!(self, OdooResponse::Success { .. })
    }
}

#[derive(Deserialize, Debug)]
struct ServerVersion {
    server_version: String,
    server_serie: String,
}

#[derive(From, Debug)]
struct PartnerId(ManyToOne);

#[derive(Debug)]
struct ManyToOne {
    id: Id,
    name: String,
}

fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: From<ManyToOne>,
    D: Deserializer<'de>,
{
    let json = serde_json::Value::deserialize(deserializer)?;
    match json {
        serde_json::Value::Array(arr) => {
            if arr.len() != 2 {
                return Err(serde::de::Error::custom(
                    "Invalid JSON structure for ManyToOne data type: Array is not of length 2",
                ));
            }
            let id = if let serde_json::Value::Number(id) = &arr[0] {
                id.as_u64().ok_or(serde::de::Error::custom("Invalid JSON structure for ManyToOne data type: First array element is not an integer"))?
            } else {
                return Err(serde::de::Error::custom(
                    "Invalid JSON structure for ManyToOne data type: First array element is not a number",
                ));
            };
            let name = if let serde_json::Value::String(name) = &arr[1] {
                name
            } else {
                return Err(serde::de::Error::custom(
                    "Invalid JSON structure for ManyToOne data type: Second element is not a string",
                ));
            };
            Ok(Some(
                ManyToOne {
                    id,
                    name: name.clone(),
                }
                .into(),
            ))
        }
        serde_json::Value::Bool(false) => Ok(None),
        _ => Err(serde::de::Error::custom(
            "Invalid JSON structure for ManyToOne data type: Cannot parse as array or 'false'",
        )),
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum SubscriptionState {
    Draft,
    Sale,
    Cancel,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Deserialize, Debug)]
struct Subscription {
    id: Id,
    recurrence_id: serde_json::Value,
    renew_state: serde_json::Value,
    state: SubscriptionState,
    #[serde(deserialize_with = "deserialize")]
    partner_id: Option<PartnerId>,
    access_url: String,
    access_token: String,
    order_line: Vec<Id>,
    invoice_ids: Vec<Id>,
}

impl Odoo {
    pub fn new(base_url: Url, db_name: String, user_id: Id, api_key: String) -> Odoo {
        Self {
            reqwest_client: reqwest::Client::new(),
            base_url,
            db_name,
            user_id,
            api_key,
        }
    }

    /// Tries to create the Odoo instance from the environment variables.
    ///
    /// *Panics:* If any error occurs
    pub fn from_env() -> Odoo {
        let base_url = env::var("ODOO_BASE_URL")
            .expect("ODOO_BASE_URL must be set")
            .parse()
            .expect("ODOO_BASE_URL must be a valid URL");
        let db_name = env::var("ODOO_DATABASE_NAME").expect("ODOO_DATABASE_NAME must be set");
        let user_id = env::var("ODOO_USER_ID")
            .expect("ODOO_USER_ID must be set")
            .parse()
            .expect("ODOO_USER_ID must be a u64");
        let api_key = env::var("ODOO_API_KEY").expect("ODOO_API_KEY must be set");
        Self::new(base_url, db_name, user_id, api_key)
    }

    async fn request<T>(&self, request: RequestBuilder) -> Result<OdooResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        let args = vec![
            self.db_name.as_str().into(),
            self.user_id.into(),
            self.api_key.as_str().into(),
            request.resource.as_str().into(),
            request.method.as_str().into(),
            request.args.into(),
            request.kwargs.into(),
        ];
        let rpc = JsonRpc {
            jsonrpc: "2.0".to_string(),
            method: "call".to_string(),
            id: 0,
            params: RpcParams {
                service: "object".to_string(),
                method: "execute_kw".to_string(),
                args,
            },
        };

        Ok(self
            .reqwest_client
            .post(self.base_url.clone())
            .json(&rpc)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn server_version(&self) -> Result<OdooResponse<ServerVersion>, Error> {
        let rpc = JsonRpc {
            jsonrpc: "2.0".to_string(),
            method: "call".to_string(),
            id: 0,
            params: RpcParams {
                service: "common".to_string(),
                method: "version".to_string(),
                args: vec![],
            },
        };

        Ok(self
            .reqwest_client
            .post(self.base_url.clone())
            .json(&rpc)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn list_subscriptions(&self) -> Result<OdooResponse<Vec<Subscription>>, Error> {
        let req = RequestBuilder {
            resource: "sale.order".to_string(),
            method: OdooMethod::SearchRead,
            args: vec![vec![vec!["is_subscription", "=", "true"]].into()],
            kwargs: Map::from_iter([(
                "fields".to_string(),
                vec![
                    "recurrence_id",
                    "renew_state",
                    "state",
                    "partner_id",
                    "access_url",
                    "access_token",
                    "order_line",
                    "invoice_ids",
                ]
                .into(),
            )]),
        };

        self.request(req).await
    }
}

#[cfg(all(test, feature = "odoo-live-tests"))]
mod test {
    use super::*;

    #[tokio::test]
    async fn server_version() {
        dotenvy::dotenv().ok();

        let odoo = Odoo::from_env();

        let server_version = odoo.server_version().await.unwrap();
        dbg!(server_version);
    }

    #[tokio::test]
    async fn list_subscriptions() {
        dotenvy::dotenv().ok();

        let odoo = Odoo::from_env();

        let subscriptions = odoo.list_subscriptions().await.unwrap();
        assert!(subscriptions.is_success());
    }
}
