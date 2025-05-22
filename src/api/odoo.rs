use crate::models::Password;
use derive_more::{Deref, Display, From};
use email_address::EmailAddress;
use http::{header, HeaderMap, HeaderValue};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use thiserror::Error;
use tracing::error;
use url::Url;
use uuid::Uuid;

struct OdooClient {
    api_key: Password,
    base_url: Url,
    client: reqwest::Client,
}

// TODO make this configurable
enum SubscriptionLevel {
    Tiny,
    Small,
    Medium,
    Large,
}

impl TryFrom<OdooProductId> for SubscriptionLevel {
    type Error = OdooError;

    fn try_from(product_id: OdooProductId) -> Result<Self, Self::Error> {
        // TODO make this mapping configurable and find out all possible values
        match product_id.0 {
            31 => Ok(Self::Medium),
            _ => Err(OdooError::InvalidMapping(format!(
                "Cannot deduce subscription level from product id: {}",
                product_id.0
            ))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooPartnerId(u32);

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooSaleOrderId(u32);

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooOrderLineId(u32);

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooProductId(u32);

#[derive(Debug, Deserialize)]
pub struct OdooPartner {
    id: OdooPartnerId,
    name: String,
    email: EmailAddress,
}

#[derive(Debug, Deserialize)]
pub struct OdooSaleOrder<OrderLine = OdooOrderLine, PartnerId = OdooPartnerId> {
    id: OdooSaleOrderId,
    partner_id: PartnerId,
    date_order: String,
    amount_total: Decimal,
    access_url: String,
    is_subscription: bool,
    subscription_state: String,
    order_line: Vec<OrderLine>,
    #[serde(deserialize_with = "deserialize_false_as_none")]
    access_token: Option<Password>,
}

fn deserialize_false_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OrFalse<Tinner> {
        T(Tinner),
        F(bool),
    }

    let inner: OrFalse<T> = OrFalse::deserialize(deserializer)?;
    match inner {
        OrFalse::T(inner) => Ok(Some(inner)),
        OrFalse::F(inner) => {
            if inner {
                Err(serde::de::Error::custom("Expected false but got true"))
            } else {
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OdooOrderLine {
    id: OdooOrderLineId,
    product_id: OdooProductId,
}

#[derive(Debug)]
enum OdooResponse<T> {
    Success { data: T, success: bool },
    Error { detail: serde_json::Value },
}

impl<'de, T> Deserialize<'de> for OdooResponse<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum OdooResponseInner {
            Success {
                data: serde_json::Value,
                success: bool,
            },
            Error {
                detail: serde_json::Value,
            },
        }

        let inner = OdooResponseInner::deserialize(deserializer)?;
        match inner {
            OdooResponseInner::Success { data, success } => Ok(OdooResponse::Success {
                data: serde_json::from_value(data).map_err(serde::de::Error::custom)?,
                success,
            }),
            OdooResponseInner::Error { detail } => Ok(OdooResponse::Error { detail }),
        }
    }
}

impl<T> From<OdooResponse<T>> for Result<T, OdooError> {
    fn from(resp: OdooResponse<T>) -> Self {
        match resp {
            OdooResponse::Success { data, success } => {
                if !success {
                    error!(
                        "Odoo API returned an successful format but the 'success' boolean is set to false"
                    );
                    return Err(OdooError::OdooApiError(Default::default()));
                }
                Ok(data)
            }
            OdooResponse::Error { detail } => Err(OdooError::OdooApiError(detail)),
        }
    }
}

#[derive(Debug, Error)]
pub enum OdooError {
    #[error("Reqwest Error in Odoo API: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Invalid Header Value: {0}")]
    InvalidHeaderValue(#[from] header::InvalidHeaderValue),
    #[error("Odoo API Error: {0}")]
    OdooApiError(serde_json::Value),
    #[error("Invalid Mapping: {0}")]
    InvalidMapping(String),
}

impl OdooClient {
    pub fn new(api_key: Password, base_url: Url) -> Result<Self, OdooError> {
        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(api_key.reveal())?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static(concat!("remails/", env!("CARGO_PKG_VERSION"))),
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            api_key,
            base_url,
            client,
        })
    }

    async fn get_partner(&self, id: OdooPartnerId) -> Result<OdooPartner, OdooError> {
        self.client
            .get(format!("{}/res.partner/{}", self.base_url, id))
            .send()
            .await?
            .json::<OdooResponse<OdooPartner>>()
            .await?
            .into()
    }

    async fn subscriptions_by_partner_id(
        &self,
        partner_id: OdooPartnerId,
    ) -> Result<Vec<OdooSaleOrder>, OdooError> {
        self.client
            .patch(format!("{}/sale.order/search_read_nested", self.base_url))
            .json(&serde_json::json!({
                "domain": [
                    [
                        "state",
                        "=",
                        "sale"
                    ],
                    [
                        "partner_id.id",
                        "=",
                        partner_id
                    ]
                ],
                "fields": [
                    "id",
                    "name",
                    "partner_id",
                    "date_order",
                    "amount_total",
                    "access_url",
                    "access_token",
                    "is_subscription",
                    "subscription_state",
                    "type_name",
                    "order_line.id",
                    "order_line.product_id",
                ]
            }))
            .send()
            .await?
            .json::<OdooResponse<Vec<OdooSaleOrder>>>()
            .await?
            .into()
    }

    async fn create_subscription(
        &self,
        partner_id: OdooPartnerId,
    ) -> Result<OdooSaleOrder<OdooOrderLineId, (OdooPartnerId, String)>, OdooError> {
        let access_token = Uuid::new_v4();

        self.client
            .post(format!("{}/sale.order", self.base_url))
            .json(&serde_json::json!({
                "partner_id": partner_id,
                "order_line": [
                  {
                    "product_id": 31,
                    "product_uom_qty": 1,
                  }
                ],
                "access_token": access_token,
                "plan_id": 4,
                "subscription_state": "1_draft",
                "state": "draft"
            }))
            .send()
            .await?
            .json::<OdooResponse<OdooSaleOrder<OdooOrderLineId, (OdooPartnerId, String)>>>()
            .await?
            .into()
    }

    async fn create_partner(
        &self,
        name: &str,
        email: EmailAddress,
    ) -> Result<OdooPartner, OdooError> {
        self.client
            .post(format!("{}/res.partner", self.base_url))
            .json(&serde_json::json!({
              "name": name,
              "email": email,
            }))
            .send()
            .await?
            .json::<OdooResponse<OdooPartner>>()
            .await?
            .into()
    }
}

#[cfg(test)]
#[cfg(feature = "live-odoo-tests")]
mod test {
    use crate::api::odoo::OdooClient;
    use std::env;

    impl OdooClient {
        fn from_env() -> Self {
            let api_key = env::var("ODOO_API_KEY").unwrap();
            let base_url = env::var("ODOO_BASE_URL").unwrap();

            Self::new(api_key.into(), base_url.parse().unwrap()).unwrap()
        }
    }

    #[tokio::test]
    async fn get_partner_happy_flow() {
        let client = OdooClient::from_env();
        let partner = client.get_partner(219.into()).await.unwrap();

        dbg!(&partner);
        assert_eq!(partner.id, 219.into());
        assert_eq!(partner.name, "API user");
    }

    #[tokio::test]
    async fn subscriptions_by_partner_id_happy_flow() {
        let client = OdooClient::from_env();
        let subscriptions = client
            .subscriptions_by_partner_id(174.into())
            .await
            .unwrap();

        dbg!(&subscriptions);
    }

    #[tokio::test]
    async fn create_partner_happy_flow() {
        let client = OdooClient::from_env();
        let partner = client
            .create_partner(
                "Odoo integration test",
                "testing@remails.com".parse().unwrap(),
            )
            .await
            .unwrap();

        dbg!(&partner);
    }

    #[tokio::test]
    async fn create_subscription_happy_flow() {
        let client = OdooClient::from_env();
        let subscription = client.create_subscription(199.into()).await.unwrap();

        dbg!(&subscription);
    }
}
