use crate::models::Password;
use derive_more::{Deref, Display, From};
use http::{HeaderMap, HeaderValue, header};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;
use url::Url;

struct OdooClient {
    api_key: Password,
    base_url: Url,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooPartnerId(u32);

#[derive(Debug, Serialize, Deserialize, Deref, Display, From, PartialEq, Eq)]
#[serde(transparent)]
pub struct OdooSaleOrderId(u32);

#[derive(Debug, Deserialize)]
pub struct OdooPartner {
    id: OdooPartnerId,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct OdooSaleOrder {
    id: OdooSaleOrderId,
    partner_id: OdooPartnerId,
    date_order: String,
    amount_total: Decimal,
    access_url: String,
    is_subscription: bool,
    subscription_state: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OdooResponse<T> {
    Success { data: T, success: bool },
    Error { detail: serde_json::Value },
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
                    "is_subscription",
                    "subscription_state",
                    "type_name",
                    "order_line.id",
                    "order_line.product_id",
                    "order_line.price_unit",
                    "order_line.product_uom_qty"
                ]
            }))
            .send()
            .await?
            .json::<OdooResponse<Vec<OdooSaleOrder>>>()
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
}
