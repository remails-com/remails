use crate::models::{OrganizationId, Password};
use chrono::{DateTime, Days, Months, NaiveDate, Utc};
use derive_more::{Deref, Display, From, FromStr};
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Type};
use std::{cmp::Ordering, str::FromStr, time::Duration};
use tracing::{debug, error, info, trace, warn};
use url::Url;

const MONEYBIRD_API_URL: &str = "https://moneybird.com/api/v2";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct MoneybirdContactId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
struct SubscriptionTemplateId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct SubscriptionId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
struct ProductId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct RecurringSalesInvoiceId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct AdministrationId(String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr, Type)]
pub struct WebhookId(String);

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
struct MoneybirdSubscription {
    id: SubscriptionId,
    contact: Contact,
    recurring_sales_invoice_id: RecurringSalesInvoiceId,
    cancelled_at: Option<DateTime<Utc>>,
    product: Product,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Product {
    id: ProductId,
    identifier: Option<ProductIdentifier>,
    title: String,
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecurringSalesInvoice {
    id: RecurringSalesInvoiceId,
    invoice_date: NaiveDate,
    last_date: Option<NaiveDate>,
    active: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Display)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum ProductIdentifier {
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

#[derive(Serialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active(Subscription),
    Expired(Subscription<NaiveDate>),
    None,
}

#[derive(Serialize, PartialEq, Debug)]
pub struct Subscription<EndDate = Option<NaiveDate>> {
    subscription_id: SubscriptionId,
    product: ProductIdentifier,
    title: String,
    description: String,
    recurring_sales_invoice_id: RecurringSalesInvoiceId,
    start_date: NaiveDate,
    end_date: EndDate,
    sales_invoices_url: Url,
}

impl PartialOrd for SubscriptionStatus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            SubscriptionStatus::Active(Subscription { end_date, .. }) => match other {
                SubscriptionStatus::Active(Subscription {
                    end_date: other_end_date,
                    ..
                }) => match (end_date, other_end_date) {
                    (Some(l), Some(r)) => l.partial_cmp(r),
                    (Some(_), None) => Some(Ordering::Less),
                    (None, Some(_)) => Some(Ordering::Greater),
                    (None, None) => Some(Ordering::Equal),
                },
                SubscriptionStatus::Expired { .. } => Some(Ordering::Greater),
                SubscriptionStatus::None => Some(Ordering::Greater),
            },
            SubscriptionStatus::Expired(Subscription { end_date, .. }) => match other {
                SubscriptionStatus::Active { .. } => Some(Ordering::Less),
                SubscriptionStatus::Expired(Subscription {
                    end_date: other_end_date,
                    ..
                }) => end_date.partial_cmp(other_end_date),
                SubscriptionStatus::None => Some(Ordering::Greater),
            },
            SubscriptionStatus::None => match other {
                SubscriptionStatus::Active { .. } | SubscriptionStatus::Expired { .. } => {
                    Some(Ordering::Less)
                }
                SubscriptionStatus::None => Some(Ordering::Equal),
            },
        }
    }
}

impl SubscriptionStatus {
    pub fn quota(&self) -> u32 {
        match self {
            SubscriptionStatus::Active(Subscription { product, .. }) => product.monthly_quota(),
            SubscriptionStatus::Expired(_) | SubscriptionStatus::None => {
                ProductIdentifier::RmlsFree.monthly_quota()
            }
        }
    }
}

impl<'a, T> From<T> for SubscriptionStatus
where
    T: IntoIterator<Item = &'a MoneybirdSubscription>,
{
    fn from(subscriptions: T) -> Self {
        let mut iterator = subscriptions.into_iter().filter_map(|s| {
            if let Some(ref identifier) = s.product.identifier {
                match identifier {
                    ProductIdentifier::Unknown(unknown) => {
                        trace!("Unknown product identifier: {}", unknown);
                        None
                    }
                    identifier => {
                        if let Some(end_date) = s.end_date {
                            if end_date < Utc::now().date_naive() {
                                return Some(SubscriptionStatus::Expired(Subscription {
                                    subscription_id: s.id.clone(),
                                    product: identifier.clone(),
                                    title: s.product.title.clone(),
                                    description: s.product.description.clone(),
                                    recurring_sales_invoice_id: s
                                        .recurring_sales_invoice_id
                                        .clone(),
                                    start_date: s.start_date,
                                    end_date,
                                    sales_invoices_url: s.contact.sales_invoices_url.clone(),
                                }));
                            }
                        }

                        Some(SubscriptionStatus::Active(Subscription {
                            subscription_id: s.id.clone(),
                            product: identifier.clone(),
                            title: s.product.title.clone(),
                            description: s.product.description.clone(),
                            recurring_sales_invoice_id: s.recurring_sales_invoice_id.clone(),
                            start_date: s.start_date,
                            end_date: s.end_date,
                            sales_invoices_url: s.contact.sales_invoices_url.clone(),
                        }))
                    }
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

struct QuotaResetInfo {
    org_id: OrganizationId,
    contact_id: Option<MoneybirdContactId>,
    quota_reset: DateTime<Utc>,
}

/// This models the content of a webhook we received from Moneybird
#[derive(Debug, Deserialize)]
pub struct MoneybirdWebhookPayload {
    // The webhook_id is not present in the `test_webhook`, otherwise it is present
    #[serde(default)]
    webhook_id: Option<WebhookId>,
    administration_id: AdministrationId,
    action: Action,
    // Strangely, the `test_webhook` does not call this `webhook_token`, but `token`
    #[serde(alias = "token")]
    webhook_token: Password,
    entity_type: EntityType,
    entity: serde_json::Value,
}

/// This models the "webhook" item returned by a `GET` request to `/webhooks`
#[derive(Debug, Deserialize)]
struct Webhook {
    id: WebhookId,
    administration_id: AdministrationId,
    url: Url,
    #[cfg(test)]
    last_http_status: Option<u16>,
    #[cfg(test)]
    last_http_body: Option<String>,
    token: Password,
}

#[derive(Debug, Deserialize)]
enum EntityType {
    Contact,
    Subscription,
    RecurringSalesInvoice,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
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

#[derive(Clone)]
pub struct MoneyBird {
    client: reqwest::Client,
    pool: PgPool,
    administration: AdministrationId,
}

impl MoneyBird {
    pub async fn new(
        api_key: Password,
        pool: PgPool,
        administration: AdministrationId,
        webhook_url: Url,
    ) -> Result<Self, Error> {
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
            .build()?;

        let res = Self {
            client,
            pool,
            administration,
        };

        let self_clone = res.clone();
        // Webhook registration must happen asynchronously, otherwise the test webhook sent by
        // moneybird will not succeed, and not webhook will get registered at all.
        tokio::spawn(async move {
            // Cannot inline this "random_delay", see: https://stackoverflow.com/a/75227719
            let random_delay = {
                let mut rng = rand::rng();
                rng.random_range(0..10)
            };

            // If multiple instances (Pods) start at the same time,
            // try to avoid race conditions by introducing a random delay
            // so that only one will register the webhook
            tokio::time::sleep(Duration::from_secs(random_delay)).await;

            self_clone
                .register_webhook(webhook_url)
                .await
                .inspect_err(|err| error!("Failed to register webhook: {}", err))
                .ok();
        });

        Ok(res)
    }

    async fn register_webhook(&self, url: Url) -> Result<(), Error> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT true AS "exists!" FROM moneybird_webhook
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(true) = exists {
            info!("Moneybird webhook already registered");
            return Ok(());
        }

        info!("registering Moneybird webhook");

        let webhook: Webhook = self
            .client
            .post(self.url("webhooks"))
            .json(&serde_json::json!({
                "url": url,
                "enabled_events": [
                    "subscription_cancelled",
                    "subscription_created",
                    "subscription_edited",
                    "subscription_resumed",
                    "subscription_updated"
                ]
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        sqlx::query!(
            r#"
            INSERT INTO moneybird_webhook (moneybird_id, token) VALUES ($1, $2)
            "#,
            *webhook.id,
            webhook.token.danger_as_str()
        )
        .execute(&self.pool)
        .await?;

        info!(
            webhook_id = webhook.id.as_str(),
            "Moneybird webhook registered"
        );

        Ok(())
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{MONEYBIRD_API_URL}/{}/{}",
            self.administration,
            path.trim_matches('/')
        )
    }

    pub async fn webhook_handler(&self, payload: MoneybirdWebhookPayload) -> Result<(), Error> {
        if matches!(payload.action, Action::TestWebhook) {
            debug!("Received test webhook");
            return Ok(());
        }

        let Some(webhook_id) = payload.webhook_id else {
            warn!("Received webhook without webhook_id");
            return Err(Error::Unauthorized);
        };

        // TODO check if we should hash the moneybird token
        let exists = sqlx::query_scalar!(
            r#"
            SELECT true AS "exists!" FROM moneybird_webhook WHERE moneybird_id = $1 AND token = $2
            "#,
            *webhook_id,
            payload.webhook_token.danger_as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            warn!("Received webhook for unknown moneybird webhook");
            return Err(Error::Unauthorized);
        };

        match payload.action {
            Action::TestWebhook => {}
            Action::Unknown(unknown) => {
                warn!("Received unknown webhook action: {unknown}");
            }
            Action::SubscriptionCancelled
            | Action::SubscriptionCreated
            | Action::SubscriptionEdited
            | Action::SubscriptionResumed
            | Action::SubscriptionUpdated => {
                trace!("received {:?} webhook", payload.action);
                let subscription = serde_json::from_value::<MoneybirdSubscription>(payload.entity)?;
                self.sync_subscription(subscription).await?;
            }
        };

        Ok(())
    }

    async fn sync_subscription(&self, subscription: MoneybirdSubscription) -> Result<(), Error> {
        debug!(
            subscription_id = subscription.id.as_str(),
            moneybird_contact_id = subscription.contact.id.as_str(),
            "syncing subscription"
        );

        struct ProdAndReset {
            subscription_product: String,
            quota_reset: DateTime<Utc>,
        }
        let product_and_reset_date = sqlx::query_as!(
            ProdAndReset,
            r#"
            SELECT subscription_product, quota_reset FROM organizations WHERE moneybird_contact_id = $1
            "#,
            *subscription.contact.id
        )
            .fetch_one(&self.pool)
            .await?;

        let current_product: ProductIdentifier =
            product_and_reset_date.subscription_product.parse().unwrap();

        let subscription_status: SubscriptionStatus = [&subscription].into();

        let quota_reset = self
            .calculate_quota_reset_datetime(
                &subscription_status,
                product_and_reset_date.quota_reset,
            )
            .await?;

        let product = match subscription_status {
            SubscriptionStatus::Active(Subscription { product, .. }) => product,
            SubscriptionStatus::Expired(_) | SubscriptionStatus::None => {
                ProductIdentifier::RmlsFree
            }
        };

        let mut delta = 0i64;
        if product != current_product {
            delta = product.monthly_quota() as i64 - current_product.monthly_quota() as i64;
            info!(
                subscription_id = subscription.id.as_str(),
                moneybird_contact_id = subscription.contact.id.as_str(),
                current_product = current_product.to_string(),
                new_product = product.to_string(),
                "Subscription product has changed -> Adjusting the remaining message quota by {delta:+}"
            );
        }
        let new_quota = sqlx::query_scalar!(
            r#"
            UPDATE organizations
            SET remaining_message_quota = remaining_message_quota + $2,
                subscription_product = $3,
                quota_reset = $4
            WHERE moneybird_contact_id = $1
            returning remaining_message_quota
            "#,
            *subscription.contact.id,
            delta,
            product.to_string(),
            quota_reset
        )
        .fetch_one(&self.pool)
        .await?;

        // Post-mortem sanity check if the new remaining quota isn't bigger than the total monthly quota
        debug_assert!(new_quota <= product.monthly_quota() as i64);
        if new_quota >= product.monthly_quota() as i64 {
            warn!(
                subscription_id = subscription.id.as_str(),
                moneybird_contact_id = subscription.contact.id.as_str(),
                current_product = current_product.to_string(),
                new_product = product.to_string(),
                "New message quota exceeds the total monthly quota"
            );
        }

        Ok(())
    }

    async fn next_invoice_date(
        &self,
        recurring_sales_invoice_id: &RecurringSalesInvoiceId,
    ) -> Result<NaiveDate, Error> {
        Ok(self
            .client
            .get(self.url(&format!(
                "recurring_sales_invoices/{}",
                recurring_sales_invoice_id
            )))
            .send()
            .await?
            .error_for_status()?
            .json::<RecurringSalesInvoice>()
            .await?
            .invoice_date)
    }

    async fn calculate_quota_reset_datetime(
        &self,
        subscription: &SubscriptionStatus,
        mut current_quota_reset: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, Error> {
        Ok(match subscription {
            SubscriptionStatus::Active(Subscription {
                                           end_date,
                                           recurring_sales_invoice_id,
                                           subscription_id,
                                           ..
                                       }) => {
                if let Some(end_date) = end_date {
                    trace!(
                        subscription_id = subscription_id.as_str(),
                        "Calculating subscription period end based on the subscription `end_date`"
                    );
                    *end_date
                } else {
                    trace!(
                        subscription_id = subscription_id.as_str(),
                        recurring_sales_invoice_id = recurring_sales_invoice_id.as_str(),
                        "Calculating subscription period end based on the next recurring invoice `invoice_date`"
                    );
                    self.next_invoice_date(recurring_sales_invoice_id).await?
                        .checked_sub_days(Days::new(1))
                        .ok_or(
                            Error::Moneybird("Could not calculate subscription period end based on the next invoice date".to_string()))?
                }
            }
            SubscriptionStatus::Expired(_) | SubscriptionStatus::None => {
                trace!("Calculating subscription period end based existing reset date as the current subscription is expired or does not exist");
                while current_quota_reset < Utc::now() {
                    current_quota_reset = current_quota_reset
                        .checked_add_months(Months::new(1))
                        .ok_or(Error::Moneybird(
                            "Could not calculate subscription period end based existing reset date"
                                .to_string(),
                        ))?;
                }
                current_quota_reset.date_naive()
            }
        }.and_hms_opt(23, 59, 59)
            .ok_or(Error::Moneybird(
                "Could not add time to subscription end".to_string()
            ))?
            .and_utc())
    }

    async fn reset_all_quotas(&self) -> Result<(), Error> {
        let quota_infos = sqlx::query_as!(
            QuotaResetInfo,
            r#"
            SELECT id AS org_id,
                   moneybird_contact_id AS "contact_id: MoneybirdContactId",
                   quota_reset
            FROM organizations
            WHERE quota_reset < now()
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        for quota_info in quota_infos {
            self.reset_single_quota(quota_info).await?;
        }

        Ok(())
    }

    async fn reset_single_quota(&self, quota_info: QuotaResetInfo) -> Result<(), Error> {
        let subscription_status = if let Some(contact_id) = quota_info.contact_id {
            self.get_subscription_status_by_contact_id(contact_id)
                .await?
        } else {
            SubscriptionStatus::None
        };

        let reset_date = self
            .calculate_quota_reset_datetime(&subscription_status, quota_info.quota_reset)
            .await?;

        let quota = subscription_status.quota();

        sqlx::query!(
            r#"
            UPDATE organizations
            SET quota_reset = $2,
                remaining_message_quota = $3
            WHERE id = $1
            "#,
            *quota_info.org_id,
            reset_date,
            quota as i64
        )
        .execute(&self.pool)
        .await?;

        Ok(())
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
            .post(self.url("contacts"))
            .json(&serde_json::json!({
                "contact": {
                    "company_name": company_name
                }
            }))
            .send()
            .await?
            .json()
            .await?;

        sqlx::query!(
            r#"
            UPDATE organizations SET moneybird_contact_id = $2 WHERE id = $1
            "#,
            *org_id,
            *contact.id
        )
        .execute(&self.pool)
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

        let sales_link: String = self
            .client
            .get(self.url(&format!(
                "subscription_templates/{}/checkout_identifier?contact_id={contact_id}",
                templates[0]
            )))
            .send()
            .await?
            .json()
            .await?;

        Ok(sales_link.parse()?)
    }

    pub async fn get_subscription_status(
        &self,
        org_id: OrganizationId,
    ) -> Result<SubscriptionStatus, Error> {
        let contact_id: Option<MoneybirdContactId> = sqlx::query_scalar!(
            r#"
            SELECT moneybird_contact_id FROM organizations WHERE id = $1
            "#,
            *org_id
        )
        .fetch_one(&self.pool)
        .await?
        .map(|id| id.into());

        let Some(contact_id) = contact_id else {
            trace!(
                organization_id = org_id.as_uuid().to_string(),
                "No moneybird contact found"
            );
            return Ok(SubscriptionStatus::None);
        };

        self.get_subscription_status_by_contact_id(contact_id).await
    }

    async fn get_subscription_status_by_contact_id(
        &self,
        contact_id: MoneybirdContactId,
    ) -> Result<SubscriptionStatus, Error> {
        let subscription_status: SubscriptionStatus = self
            .client
            .get(self.url(&format!("subscriptions?contact_id={contact_id}",)))
            .send()
            .await?
            .json::<Vec<MoneybirdSubscription>>()
            .await?
            .as_slice()
            .into();

        Ok(subscription_status)
    }

    async fn subscription_templates(&self) -> Result<Vec<SubscriptionTemplate>, Error> {
        Ok(self
            .client
            .get(self.url("subscription_templates"))
            .send()
            .await?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::{OrganizationFilter, OrganizationRepository};
    use chrono::Datelike;

    impl<T> Default for Subscription<T>
    where
        T: Default,
    {
        fn default() -> Self {
            Self {
                subscription_id: SubscriptionId("SubscriptionId".to_string()),
                product: ProductIdentifier::RmlsFree,
                title: "".to_string(),
                description: "".to_string(),
                recurring_sales_invoice_id: RecurringSalesInvoiceId("InvoiceId".to_string()),
                start_date: Utc::now().date_naive(),
                end_date: T::default(),
                sales_invoices_url: "https://locahost".parse().unwrap(),
            }
        }
    }

    #[test]
    fn subscription_ordering() {
        let active_none = SubscriptionStatus::Active(Subscription {
            end_date: None,
            ..Default::default()
        });
        let active_today = SubscriptionStatus::Active(Subscription {
            end_date: Some(Utc::now().date_naive()),
            ..Default::default()
        });
        let active_tomorrow = SubscriptionStatus::Active(Subscription {
            end_date: Some(
                Utc::now()
                    .date_naive()
                    .checked_add_days(Days::new(1))
                    .unwrap(),
            ),
            ..Default::default()
        });

        let expired_yesterday = SubscriptionStatus::Expired(Subscription {
            end_date: Utc::now()
                .date_naive()
                .checked_sub_days(Days::new(1))
                .unwrap(),
            ..Default::default()
        });

        let expired_two_days_ago = SubscriptionStatus::Expired(Subscription {
            end_date: Utc::now()
                .date_naive()
                .checked_sub_days(Days::new(2))
                .unwrap(),
            ..Default::default()
        });

        let none = SubscriptionStatus::None;

        assert!(active_none > active_tomorrow);
        assert!(active_tomorrow < active_none);
        assert!(active_tomorrow > active_today);
        assert!(active_none > active_today);

        assert!(active_today > expired_yesterday);
        assert!(active_none > expired_two_days_ago);

        assert!(expired_yesterday > expired_two_days_ago);

        assert!(active_none > none);
        assert!(none < active_none);
        assert!(active_today > none);
        assert!(active_tomorrow > none);

        assert!(expired_yesterday > none);
        assert!(none < expired_yesterday);
    }

    #[sqlx::test(fixtures("organizations"))]
    async fn quota_reset_without_moneybird(db: PgPool) {
        let moneybird = MoneyBird::new(
            "not-an-actual-api-key".to_string().into(),
            db.clone(),
            "not-an-actual-administration".to_string().into(),
            "https://dump.tweede.golf/dump/moneybird".parse().unwrap(),
        )
        .await
        .unwrap();
        moneybird.reset_all_quotas().await.unwrap();

        let org_repo = OrganizationRepository::new(db);
        let orgs = org_repo
            .list(&OrganizationFilter { orgs: None })
            .await
            .unwrap();

        for org in orgs {
            match org.id().as_uuid().to_string().as_str() {
                "44729d9f-a7dc-4226-b412-36a7537f5176" => {
                    assert_eq!(org.message_quota(), 5_000);
                    assert_eq!(
                        org.quota_reset().date_naive(),
                        Utc::now()
                            .date_naive()
                            .checked_add_months(Months::new(1))
                            .unwrap()
                    );
                }
                "5d55aec5-136a-407c-952f-5348d4398204" => {
                    assert_eq!(org.message_quota(), 500);
                    assert_eq!(
                        org.quota_reset().date_naive(),
                        Utc::now()
                            .date_naive()
                            .checked_add_months(Months::new(1))
                            .unwrap()
                    );
                }
                "533d9a19-16e8-4a1b-a824-ff50af8b428c" => {
                    assert_eq!(org.message_quota(), 1_000);
                    assert_eq!(
                        org.quota_reset().date_naive(),
                        Utc::now()
                            .date_naive()
                            .checked_add_months(Months::new(1))
                            .unwrap()
                    );
                }
                "ee14cdb8-f62e-42ac-a0cd-294d708be994" => {
                    assert_eq!(org.message_quota(), 1_000);
                    let new_reset_date = if Utc::now().date_naive().day() <= 25 {
                        Utc::now().date_naive().with_day(25).unwrap()
                    } else {
                        Utc::now()
                            .date_naive()
                            .with_day(25)
                            .unwrap()
                            .checked_add_months(Months::new(1))
                            .unwrap()
                    };
                    assert_eq!(org.quota_reset().date_naive(), new_reset_date);
                }
                "7b2d91d0-f9d9-4ddd-88ac-6853f736501c" => {
                    assert_eq!(org.message_quota(), 333);
                    assert_eq!(org.quota_reset().date_naive(), Utc::now().date_naive());
                }
                "0f83bfee-e7b6-4670-83ec-192afec2b137" => {
                    assert_eq!(org.message_quota(), 1_000);
                    // If a reset date is the last of the month,
                    // it will gradually reduce to the 28th of the month, because of the February.
                    // This is only true if there is no connection to Moneybird,
                    // as with Moneybird will reset the quota on the last day before the new invoice,
                    // which will potentially be the last day of the month.
                    assert_eq!(org.quota_reset().day(), 28u32)
                }
                _ => panic!("Unexpected organization id"),
            }
        }
    }
}
