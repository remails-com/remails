use crate::models::Password;
use chrono::{DateTime, NaiveDate, Utc};
use derive_more::{Deref, Display, From, FromStr};
use garde::Validate;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
use tracing::warn;
use url::Url;
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type, ToSchema,
)]
pub struct MoneybirdContactId(pub(super) String);

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type, ToSchema,
)]
pub(super) struct SubscriptionTemplateId(pub(super) String);

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type, ToSchema,
)]
pub struct SubscriptionId(pub(super) String);

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type, ToSchema,
)]
pub struct ProductId(pub(super) String);

#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type, ToSchema,
)]
pub(super) struct RecurringSalesInvoiceId(pub(super) String);

#[derive(
    Debug,
    Clone,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    FromStr,
    Type,
    ToSchema,
    Validate,
)]
pub(super) struct AdministrationId(#[garde(length(min = 1, max = 100))] pub(super) String);

#[derive(
    Debug,
    Clone,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    FromStr,
    Type,
    ToSchema,
    Validate,
)]
pub(super) struct WebhookId(#[garde(length(min = 1, max = 100))] pub(super) String);

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
pub struct Product {
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Display, ToSchema)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum ProductIdentifier {
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
                Self::NotSubscribed
            }
        })
    }
}

impl ProductIdentifier {
    pub fn monthly_quota(&self) -> u32 {
        match self {
            ProductIdentifier::NotSubscribed => 0,
            ProductIdentifier::RmlsFree => 3_000,
            ProductIdentifier::RmlsTinyMonthly => 100_000,
            ProductIdentifier::RmlsSmallMonthly => 300_000,
            ProductIdentifier::RmlsMediumMonthly => 700_000,
            ProductIdentifier::RmlsLargeMonthly => 1_500_000,
            ProductIdentifier::RmlsTinyYearly => 100_000,
            ProductIdentifier::RmlsSmallYearly => 300_000,
            ProductIdentifier::RmlsMediumYearly => 700_000,
            ProductIdentifier::RmlsLargeYearly => 1_500_000,
        }
    }
}

#[derive(Serialize, PartialEq, Debug, Deserialize, Clone, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    #[schema(title = "Active")]
    Active(Subscription<Option<NaiveDate>>),
    #[schema(title = "Expired")]
    Expired(Subscription<NaiveDate>),
    #[schema(title = "None")]
    None,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, ToSchema)]
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

impl Subscription {
    pub fn id(&self) -> &SubscriptionId {
        &self.subscription_id
    }
    pub fn product_id(&self) -> &ProductIdentifier {
        &self.product
    }
}

/// This models the content of a webhook we received from Moneybird
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct MoneybirdWebhookPayload {
    /// The webhook_id is not present in the `test_webhook`, otherwise it is present
    #[serde(default)]
    #[garde(dive)]
    pub(super) webhook_id: Option<WebhookId>,
    #[garde(dive)]
    pub(super) administration_id: AdministrationId,
    #[garde(dive)]
    pub(super) action: Action,
    // Strangely, the `test_webhook` does not call this `webhook_token`, but `token`
    #[serde(alias = "token")]
    #[garde(dive)]
    #[schema(min_length = 6, max_length = 256)]
    pub(super) webhook_token: Password,
    #[garde(dive)]
    pub(super) entity_type: EntityType,
    #[garde(skip)]
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

#[derive(Debug, Deserialize, PartialEq, Eq, ToSchema, Validate)]
pub(super) enum EntityType {
    Contact,
    Subscription,
    RecurringSalesInvoice,
    #[serde(untagged)]
    Unknown(#[garde(length(min = 1, max = 100))] String),
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    SubscriptionCancelled,
    SubscriptionCreated,
    SubscriptionEdited,
    SubscriptionResumed,
    SubscriptionUpdated,
    TestWebhook,
    #[serde(untagged)]
    Unknown(#[garde(length(min = 1, max = 100))] String),
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
    #[error("Precondition failed: {0}")]
    PreconditionFailed(&'static str),
}
