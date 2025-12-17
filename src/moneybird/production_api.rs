use crate::moneybird::{
    Error, MONEYBIRD_API_URL, MoneybirdApi, Webhook,
    model::{
        AdministrationId, Contact, MoneybirdContactId, MoneybirdSubscription,
        RecurringSalesInvoice, RecurringSalesInvoiceId, SubscriptionStatus, SubscriptionTemplate,
    },
};
use async_trait::async_trait;
use chrono::NaiveDate;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use tracing::{error, warn};
use url::Url;

#[derive(Clone)]
pub(super) struct ProductionMoneybirdApi {
    client: reqwest::Client,
    administration: AdministrationId,
    webhook_url: Url,
}

impl ProductionMoneybirdApi {
    pub(super) fn new(
        api_key: String,
        administration: AdministrationId,
        webhook_url: Url,
    ) -> Result<Self, Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {api_key}").parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(ACCEPT, "application/json".parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(ProductionMoneybirdApi {
            client,
            administration,
            webhook_url,
        })
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{MONEYBIRD_API_URL}/{}/{}",
            self.administration,
            path.trim_matches('/')
        )
    }
}

#[async_trait]
impl MoneybirdApi for ProductionMoneybirdApi {
    async fn register_webhook(&self) -> Option<Webhook> {
        match self
            .client
            .post(self.url("webhooks"))
            .json(&serde_json::json!({
                "url": self.webhook_url.as_str(),
                "enabled_events": [
                    "subscription_cancelled",
                    "subscription_created",
                    "subscription_edited",
                    "subscription_resumed",
                    "subscription_updated"
                ]
            }))
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => res.json().await.unwrap_or_else(|err| {
                error!("Error registering Moneybird webhook: {}", err);
                None
            }),
            Err(err) => {
                error!("Error registering Moneybird webhook: {}", err);
                None
            }
            Ok(res) => {
                error!(
                    "Error registering Moneybird webhook: Status {}, Response: {}",
                    res.status(),
                    res.text().await.unwrap()
                );
                None
            }
        }
    }

    async fn next_invoice_date(
        &self,
        recurring_sales_invoice_id: &RecurringSalesInvoiceId,
    ) -> Result<NaiveDate, Error> {
        Ok(self
            .client
            .get(self.url(&format!(
                "recurring_sales_invoices/{recurring_sales_invoice_id}",
            )))
            .send()
            .await?
            .error_for_status()?
            .json::<RecurringSalesInvoice>()
            .await?
            .invoice_date)
    }

    async fn create_contact(&self, company_name: &str) -> reqwest::Result<Contact> {
        self.client
            .post(self.url("contacts"))
            .json(&serde_json::json!({
                "contact": {
                    "company_name": company_name
                }
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    async fn subscription_templates(&self) -> Result<Vec<SubscriptionTemplate>, Error> {
        Ok(self
            .client
            .get(self.url("subscription_templates"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
    async fn get_subscription_status_by_contact_id(
        &self,
        contact_id: &MoneybirdContactId,
    ) -> Result<SubscriptionStatus, Error> {
        let subscription_status: SubscriptionStatus = self
            .client
            .get(self.url(&format!("subscriptions?contact_id={contact_id}",)))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<MoneybirdSubscription>>()
            .await?
            .as_slice()
            .into();

        Ok(subscription_status)
    }

    async fn create_sales_link(
        &self,
        moneybird_contact_id: MoneybirdContactId,
    ) -> Result<Url, Error> {
        let templates = self
            .subscription_templates()
            .await?
            .into_iter()
            .map(|t| t.id)
            .collect::<Vec<_>>();

        if templates.is_empty() {
            Err(Error::Moneybird(
                "No subscription templates found".to_string(),
            ))?
        }
        if templates.len() > 1 {
            warn!(
                "Found multiple subscription templates, using the first one: {}",
                templates[0].as_str()
            );
        }

        let sales_link: String = self
            .client
            .get(self.url(&format!(
                "subscription_templates/{}/checkout_identifier?contact_id={moneybird_contact_id}",
                templates[0]
            )))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(sales_link.parse()?)
    }

    async fn customer_contact_portal(
        &self,
        moneybird_contact_id: MoneybirdContactId,
    ) -> Result<Url, Error> {
        let link: Url = self
            .client
            .get(self.url(&format!("customer_contact_portal/{moneybird_contact_id}")))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(link)
    }
}
