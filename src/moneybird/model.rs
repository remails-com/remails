use crate::models::Password;
use chrono::{DateTime, NaiveDate, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
use tracing::warn;
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct MoneybirdContactId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct SubscriptionTemplateId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct SubscriptionId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct ProductId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct RecurringSalesInvoiceId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct AdministrationId(pub(super) String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub(super) struct WebhookId(pub(super) String);

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Contact {
    pub(super) id: MoneybirdContactId,
    pub(super) company_name: String,
    pub(super) email: String,
    pub(super) phone: String,
    pub(super) address1: String,
    pub(super) address2: String,
    pub(super) zipcode: String,
    pub(super) city: String,
    pub(super) country: String,
    pub(super) sales_invoices_url: Url,
    pub(super) contact_people: Vec<CompanyContactPerson>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct CompanyContactPerson {
    pub(super) firstname: String,
    pub(super) lastname: String,
    pub(super) phone: String,
    pub(super) email: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SubscriptionTemplate {
    pub(super) id: SubscriptionTemplateId,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct MoneybirdSubscription {
    pub(super) id: SubscriptionId,
    pub(super) contact: Contact,
    pub(super) recurring_sales_invoice_id: RecurringSalesInvoiceId,
    pub(super) cancelled_at: Option<DateTime<Utc>>,
    pub(super) product: Product,
    pub(super) start_date: NaiveDate,
    pub(super) end_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Product {
    pub(super) id: ProductId,
    pub(super) identifier: Option<ProductIdentifier>,
    pub(super) title: String,
    pub(super) description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct RecurringSalesInvoice {
    pub(super) id: RecurringSalesInvoiceId,
    pub(super) invoice_date: NaiveDate,
    pub(super) last_date: Option<NaiveDate>,
    pub(super) active: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Display)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub(super) enum ProductIdentifier {
    NotSubscribed,
    RmlsFree,
    RmlsTinyMonthly,
    RmlsSmallMonthly,
    RmlsMediumMonthly,
    RmlsLargeMonthly,
    RmlsTinyYearly,
    RmlsSmallYearly,
    RmlsMediumYearly,
    RmlsLargeYearly,
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for ProductIdentifier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "RmlsFree" => Self::RmlsFree,
            "RmlsTinyMonthly" => Self::RmlsTinyMonthly,
            "RmlsSmallMonthly" => Self::RmlsSmallMonthly,
            "RmlsMediumMonthly" => Self::RmlsMediumMonthly,
            "RmlsLargeMonthly" => Self::RmlsLargeMonthly,
            "RmlsTinyYearly" => Self::RmlsTinyYearly,
            "RmlsSmallYearly" => Self::RmlsSmallYearly,
            "RmlsMediumYearly" => Self::RmlsMediumYearly,
            "RmlsLargeYearly" => Self::RmlsLargeYearly,
            unknown => {
                warn!("Unknown product identifier: {}", unknown);
                Self::Unknown(unknown.to_string())
            }
        })
    }
}

impl ProductIdentifier {
    pub fn monthly_quota(&self) -> u32 {
        match self {
            ProductIdentifier::NotSubscribed => 0,
            ProductIdentifier::RmlsFree => 1_000,
            ProductIdentifier::RmlsTinyMonthly => 100_000,
            ProductIdentifier::RmlsSmallMonthly => 300_000,
            ProductIdentifier::RmlsMediumMonthly => 700_000,
            ProductIdentifier::RmlsLargeMonthly => 1_500_000,
            ProductIdentifier::RmlsTinyYearly => 100_000,
            ProductIdentifier::RmlsSmallYearly => 300_000,
            ProductIdentifier::RmlsMediumYearly => 700_000,
            ProductIdentifier::RmlsLargeYearly => 1_500_000,
            ProductIdentifier::Unknown(_) => 0,
        }
    }
}

#[derive(Serialize, PartialEq, Debug, Deserialize, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active(Subscription),
    Expired(Subscription<NaiveDate>),
    None,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Subscription<EndDate = Option<NaiveDate>> {
    pub(super) subscription_id: SubscriptionId,
    pub(super) product: ProductIdentifier,
    pub(super) title: String,
    pub(super) description: String,
    pub(super) recurring_sales_invoice_id: RecurringSalesInvoiceId,
    pub(super) start_date: NaiveDate,
    pub(super) end_date: EndDate,
    pub(super) sales_invoices_url: Url,
}

/// This models the content of a webhook we received from Moneybird
#[derive(Debug, Deserialize)]
pub struct MoneybirdWebhookPayload {
    // The webhook_id is not present in the `test_webhook`, otherwise it is present
    #[serde(default)]
    pub(super) webhook_id: Option<WebhookId>,
    pub(super) administration_id: AdministrationId,
    pub(super) action: Action,
    // Strangely, the `test_webhook` does not call this `webhook_token`, but `token`
    #[serde(alias = "token")]
    pub(super) webhook_token: Password,
    pub(super) entity_type: EntityType,
    pub(super) entity: serde_json::Value,
}

/// This models the "webhook" item returned by a `GET` request to `/webhooks`
#[derive(Debug, Deserialize)]
pub(super) struct Webhook {
    pub(super) id: WebhookId,
    pub(super) administration_id: AdministrationId,
    pub(super) url: Url,
    pub(super) token: Password,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(super) enum EntityType {
    Contact,
    Subscription,
    RecurringSalesInvoice,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    SubscriptionCancelled,
    SubscriptionCreated,
    SubscriptionEdited,
    SubscriptionResumed,
    SubscriptionUpdated,
    TestWebhook,
    #[serde(untagged)]
    Unknown(String),
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
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
}
