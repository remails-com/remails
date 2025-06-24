use crate::models::{OrganizationId, Password};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Type};
use tracing::{info, trace, warn};
use url::Url;

const MONEYBIRD_API_URL: &str = "https://moneybird.com/api/v2/";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct MoneybirdContactId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
struct SubscriptionTemplateId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
struct SubscriptionId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
struct ProductId(String);

#[derive(Debug, Deserialize, Serialize)]
struct Contact {
    id: MoneybirdContactId,
    company_name: String,
    email: String,
    phone: String,
    address1: String,
    address2: String,
    zipcode: String,
    city: String,
    country: String,
    sales_invoices_url: Url,
    contact_people: Vec<CompanyContactPerson>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompanyContactPerson {
    firstname: String,
    lastname: String,
    phone: String,
    email: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct SubscriptionTemplate {
    id: SubscriptionTemplateId,
}

#[derive(Debug, Deserialize, Serialize)]
struct Subscription {
    id: SubscriptionId,
    cancelled_at: Option<DateTime<Utc>>,
    product: Product,
    start_date: DateTime<Utc>,
    end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Product {
    id: ProductId,
    identifier: Option<ProductIdentifier>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
enum ProductIdentifier {
    RmlsFree,
    RmlsTinyMonthly,
    RmlsSmallMonthly,
    RmlsMediumMonthly,
    RmlsLargeMonthly,
    RmlsTinyYearly,
    RmlsSmallYearly,
    RmlsMediumYearly,
    RmlsLargeYearly,
    Unknown(String),
}

pub enum SubscriptionStatus {
    Active {
        product: ProductIdentifier,
        until: Option<DateTime<Utc>>,
    },
    None,
}

impl From<&[Subscription]> for SubscriptionStatus {
    fn from(subscriptions: &[Subscription]) -> Self {
        let mut iterator = subscriptions.iter().filter_map(|s| {
            if let Some(ref identifier) = s.product.identifier {
                match identifier {
                    ProductIdentifier::Unknown(unknown) => {
                        trace!("Unknown product identifier: {}", unknown);
                        None
                    }
                    identifier => Some(SubscriptionStatus::Active {
                        product: identifier.clone(),
                        until: s.end_date,
                    }),
                }
            } else {
                None
            }
        });
        let subscription = iterator.next();
        if iterator.next().is_some() {
            warn!("Found multiple subscriptions");
        }

        subscription.unwrap_or(SubscriptionStatus::None)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Moneybird error: {0}")]
    Moneybird(String),
    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Parse url error: {0}")]
    ParseUrl(#[from] url::ParseError),
}

pub struct MoneyBird {
    client: reqwest::Client,
    pool: PgPool,
}

impl MoneyBird {
    pub fn new(api_key: Password, pool: PgPool) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key.danger_as_str())
                .parse()
                .unwrap(),
        );
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(ACCEPT, "application/json".parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client, pool }
    }

    async fn create_contact(&self, org_id: OrganizationId) -> Result<Contact, Error> {
        info!(
            organization_id = org_id.as_uuid().to_string(),
            "Creating new moneybird contact for organization"
        );

        let company_name = sqlx::query_scalar!(
            r#"
            SELECT name FROM organizations WHERE id = $1
            "#,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?;

        let contact: Contact = self
            .client
            .post(format!("{MONEYBIRD_API_URL}/contacts"))
            .json(&serde_json::json!({
                "contact": {
                    "company_name": company_name
                }
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(contact)
    }

    async fn get_contact_id(&self, org_id: OrganizationId) -> Result<MoneybirdContactId, Error> {
        let contact_id: Option<MoneybirdContactId> = sqlx::query_scalar!(
            r#"
            SELECT moneybird_contact_id FROM organizations WHERE id = $1
            "#,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?
        .map(|id| id.into());

        if let Some(contact_id) = contact_id {
            trace!(
                organization_id = org_id.as_uuid().to_string(),
                contact_id = contact_id.as_str(),
                "Found existing moneybird contact"
            );
            return Ok(contact_id);
        };

        Ok(self.create_contact(org_id).await?.id)
    }

    pub async fn create_sales_link(&self, org_id: OrganizationId) -> Result<Url, Error> {
        let contact_id = self.get_contact_id(org_id).await?;

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

        let sales_link: String = self.client
            .get(format!("{MONEYBIRD_API_URL}/subscription_templates/{}/checkout_identifier?contact_id={contact_id}", templates[0]))
            .send().await?.json().await?;

        Ok(sales_link.parse()?)
    }

    pub async fn get_subscription_status(
        &self,
        org_id: OrganizationId,
    ) -> Result<SubscriptionStatus, Error> {
        let contact_id: Option<MoneybirdContactId> = sqlx::query_scalar!(
            r#"
            SELECT moneybird_contact_id AS "moneybird_contact_id: MoneybirdContactId" FROM organizations WHERE id = $1
            "#,
            *org_id
        )
            .fetch_one(&self.pool)
            .await?;

        let Some(contact_id) = contact_id else {
            trace!(
                organization_id = org_id.as_uuid().to_string(),
                "No moneybird contact found"
            );
            return Ok(SubscriptionStatus::None);
        };

        let subscription_status: SubscriptionStatus = self
            .client
            .get(format!(
                "{MONEYBIRD_API_URL}/subscriptions?contact_id={contact_id}"
            ))
            .send()
            .await?
            .json::<Vec<Subscription>>()
            .await?
            .as_slice()
            .into();

        Ok(subscription_status)
    }

    async fn subscription_templates(&self) -> Result<Vec<SubscriptionTemplate>, Error> {
        Ok(self
            .client
            .get(format!("{MONEYBIRD_API_URL}/subscription_templates"))
            .send()
            .await?
            .json()
            .await?)
    }
}
