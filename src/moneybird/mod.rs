mod mock;
mod model;
mod production_api;

pub use model::*;

use crate::{
    Environment,
    models::OrganizationId,
    moneybird::{mock::MockMoneybirdApi, production_api::ProductionMoneybirdApi},
};
use async_trait::async_trait;
use chrono::{DateTime, Days, NaiveDate, Utc};
#[cfg(not(test))]
use rand::Rng;
use sqlx::PgPool;
#[cfg(not(test))]
use std::time::Duration;
use std::{cmp::Ordering, env, sync::Arc};
use tracing::{debug, error, info, trace, warn};
use url::Url;

const MONEYBIRD_API_URL: &str = "https://moneybird.com/api/v2";

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
                ProductIdentifier::NotSubscribed.monthly_quota()
            }
        }
    }

    fn subscription_id(&self) -> Option<&SubscriptionId> {
        match self {
            SubscriptionStatus::Active(Subscription {
                subscription_id, ..
            }) => Some(subscription_id),
            SubscriptionStatus::Expired(Subscription {
                subscription_id, ..
            }) => Some(subscription_id),
            SubscriptionStatus::None => None,
        }
    }

    fn status_string(&self) -> &'static str {
        match self {
            SubscriptionStatus::Active(_) => "active",
            SubscriptionStatus::Expired(_) => "expired",
            SubscriptionStatus::None => "none",
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
                        if let Some(end_date) = s.end_date
                            && end_date < Utc::now().date_naive()
                        {
                            return Some(SubscriptionStatus::Expired(Subscription {
                                subscription_id: s.id.clone(),
                                product: identifier.clone(),
                                title: s.product.title.clone(),
                                description: s.product.description.clone(),
                                recurring_sales_invoice_id: s.recurring_sales_invoice_id.clone(),
                                start_date: s.start_date,
                                end_date,
                                sales_invoices_url: s.contact.sales_invoices_url.clone(),
                            }));
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

#[derive(Clone)]
pub struct MoneyBird {
    api: Arc<dyn MoneybirdApi + Send + Sync>,
    pool: PgPool,
}

#[async_trait]
trait MoneybirdApi {
    async fn register_webhook(&self) -> Option<Webhook>;
    async fn next_invoice_date(
        &self,
        recurring_sales_invoice_id: &RecurringSalesInvoiceId,
    ) -> Result<NaiveDate, Error>;
    async fn create_contact(&self, company_name: &str) -> reqwest::Result<Contact>;
    async fn subscription_templates(&self) -> Result<Vec<SubscriptionTemplate>, Error>;
    async fn get_subscription_status_by_contact_id(
        &self,
        contact_id: &MoneybirdContactId,
    ) -> Result<SubscriptionStatus, Error>;
    async fn create_sales_link(
        &self,
        moneybird_contact_id: MoneybirdContactId,
    ) -> Result<Url, Error>;
}

impl MoneyBird {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let api_key = env::var("MONEYBIRD_API_KEY").ok();

        let administration = env::var("MONEYBIRD_ADMINISTRATION_ID").ok().map(Into::into);

        let webhook_url = env::var("MONEYBIRD_WEBHOOK_URL")
            .ok()
            .and_then(|url| {
                if url.trim().is_empty() {
                    None
                } else {
                    Some(url)
                }
            })
            .map(|url| {
                url.parse()
                    .expect("MONEYBIRD_WEBHOOK_URL env var must be a valid URL")
            });

        let environment: Environment = env::var("ENVIRONMENT")
            .map(|s| s.parse())
            .inspect_err(|_| warn!("Did not find ENVIRONMENT env var, defaulting to development"))
            .unwrap_or(Ok(Environment::Development))
            .expect(
                "Invalid ENVIRONMENT env var, must be one of: development, production, or staging",
            );

        let api: Arc<dyn MoneybirdApi + Send + Sync> = match (
            api_key,
            administration,
            webhook_url,
            environment,
        ) {
            (Some(api_key), Some(administration), Some(webhook_url), Environment::Production) => {
                Arc::new(ProductionMoneybirdApi::new(
                    api_key,
                    administration,
                    webhook_url,
                )?)
            }
            (Some(api_key), Some(administration), Some(webhook_url), _) => {
                warn!("Using production Moneybird API even though not in production environment");
                Arc::new(ProductionMoneybirdApi::new(
                    api_key,
                    administration,
                    webhook_url,
                )?)
            }
            (_, _, _, Environment::Production) => {
                panic!(
                    "Missing at least one of the following environment variables as you are in production environment: MONEYBIRD_API_KEY, MONEYBIRD_ADMINISTRATION_ID, MONEYBIRD_WEBHOOK_URL"
                );
            }
            (_, _, _, _) => {
                warn!("Using Mock Moneybird API");
                info!(
                    "To use production Moneybird API make sure to set MONEYBIRD_API_KEY, MONEYBIRD_ADMINISTRATION_ID and MONEYBIRD_WEBHOOK_URL env vars"
                );
                Arc::new(MockMoneybirdApi {})
            }
        };

        let res = Self { api, pool };

        Ok(res)
    }

    /// Asynchronously register a webhook at moneybird.
    /// This function will immediately return and register the webhook in the background,
    /// logging possible errors.
    pub(crate) fn register_webhook(&self) {
        let self_clone = self.clone();
        tokio::spawn(async move {
            #[cfg(not(test))]
            {
                // Cannot inline this "random_delay", see: https://stackoverflow.com/a/75227719
                let random_delay = {
                    let mut rng = rand::rng();
                    rng.random_range(0..10)
                };

                // If multiple instances (Pods) start at the same time,
                // try to avoid race conditions by introducing a random delay
                // so that only one will register the webhook
                tokio::time::sleep(Duration::from_secs(random_delay)).await;
            }

            match sqlx::query_scalar!(
                r#"
                SELECT true AS "exists!" FROM moneybird_webhook
                "#
            )
            .fetch_optional(&self_clone.pool)
            .await
            {
                Ok(Some(true)) => {
                    info!("Moneybird webhook already registered");
                    return;
                }
                Err(err) => {
                    warn!(
                        "Error checking if moneybird webhook is already registered: {}",
                        err
                    );
                    return;
                }
                _ => {}
            };

            info!("registering Moneybird webhook");

            let Some(webhook) = self_clone.api.register_webhook().await else {
                return;
            };

            if let Err(err) = sqlx::query!(
                r#"
                INSERT INTO moneybird_webhook (moneybird_id, token_hash) VALUES ($1, $2)
                "#,
                *webhook.id,
                webhook.token.generate_hash()
            )
            .execute(&self_clone.pool)
            .await
            {
                error!("Error storing Moneybird webhook in database: {}", err);
                return;
            };

            info!(
                administration_id = webhook.administration_id.as_str(),
                webhook_id = webhook.id.as_str(),
                url = webhook.url.as_str(),
                "Moneybird webhook registered"
            );
        });
    }

    async fn authorize_webhook_call(
        &self,
        payload: &MoneybirdWebhookPayload,
    ) -> Result<WebhookId, Error> {
        let Some(webhook_id) = payload.webhook_id.clone() else {
            warn!(
                administration_id = payload.administration_id.as_str(),
                "Received webhook without webhook_id"
            );
            return Err(Error::Unauthorized);
        };

        let token_hash = sqlx::query_scalar!(
            r#"
            SELECT token_hash FROM moneybird_webhook WHERE moneybird_id = $1
            "#,
            *webhook_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(hash) = token_hash else {
            warn!(
                webhook_id = webhook_id.as_str(),
                administration_id = payload.administration_id.as_str(),
                "Received webhook for unknown moneybird webhook"
            );
            return Err(Error::Unauthorized);
        };

        payload
            .webhook_token
            .verify_password(&hash)
            .inspect_err(|err| {
                warn!(
                    webhook_id = webhook_id.as_str(),
                    administration_id = payload.administration_id.as_str(),
                    "Received webhook with invalid token: {}",
                    err
                )
            })
            .map_err(|_| Error::Unauthorized)?;

        Ok(webhook_id)
    }

    pub async fn webhook_handler(&self, payload: MoneybirdWebhookPayload) -> Result<(), Error> {
        if matches!(payload.action, Action::TestWebhook) {
            debug!(
                administration_id = payload.administration_id.as_str(),
                "Received test webhook"
            );
            return Ok(());
        }

        let webhook_id = self.authorize_webhook_call(&payload).await?;

        match payload.action {
            Action::TestWebhook => {
                unreachable!("Test webhook should have been handled before this point")
            }
            Action::Unknown(unknown) => {
                warn!(
                    webhook_id = webhook_id.as_str(),
                    administration_id = payload.administration_id.as_str(),
                    "Received unknown webhook action: {unknown}"
                );
            }
            Action::SubscriptionCancelled
            | Action::SubscriptionCreated
            | Action::SubscriptionEdited
            | Action::SubscriptionResumed
            | Action::SubscriptionUpdated => {
                self.sync_subscription(payload, webhook_id).await?;
            }
        };

        Ok(())
    }

    async fn sync_subscription(
        &self,
        payload: MoneybirdWebhookPayload,
        webhook_id: WebhookId,
    ) -> Result<(), Error> {
        debug!(
            webhook_id = webhook_id.as_str(),
            administration_id = payload.administration_id.as_str(),
            "received {:?} webhook",
            payload.action
        );
        if payload.entity_type != EntityType::Subscription {
            error!(
                webhook_id = webhook_id.as_str(),
                administration_id = payload.administration_id.as_str(),
                "webhook does not have subscription entity type"
            );
            return Err(Error::Moneybird(
                "Webhook does not have subscription entity type".to_string(),
            ));
        }
        let subscription = serde_json::from_value::<MoneybirdSubscription>(payload.entity)?;

        let Some(organization_id) = sqlx::query_scalar!(
            r#"
            SELECT id FROM organizations WHERE moneybird_contact_id = $1
            "#,
            subscription.contact.id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            warn!(
                subscription_id = subscription.id.as_str(),
                moneybird_contact_id = subscription.contact.id.as_str(),
                "Could not find organization with moneybird contact id"
            );
            return Err(Error::Moneybird(
                "Could not find organization with moneybird contact id".to_string(),
            ));
        };

        debug!(
            subscription_id = subscription.id.as_str(),
            moneybird_contact_id = subscription.contact.id.as_str(),
            "syncing subscription"
        );

        let subscription_status: SubscriptionStatus = [&subscription].into();

        self.store_subscription_status(&subscription_status, &organization_id.into())
            .await
    }

    async fn store_subscription_status(
        &self,
        subscription_status: &SubscriptionStatus,
        organization_id: &OrganizationId,
    ) -> Result<(), Error> {
        let quota_reset = self
            .calculate_quota_reset_datetime(subscription_status)
            .await?;

        let product = match subscription_status {
            SubscriptionStatus::Active(Subscription { product, .. }) => product,
            SubscriptionStatus::Expired(_) | SubscriptionStatus::None => {
                &ProductIdentifier::NotSubscribed
            }
        };

        self.make_user_admin_on_first_subscription(subscription_status, organization_id)
            .await?;

        sqlx::query!(
            r#"
            UPDATE organizations
            SET total_message_quota = $2,
                quota_reset = $3,
                current_subscription = $4
            WHERE id = $1
            "#,
            **organization_id,
            product.monthly_quota() as i64,
            quota_reset,
            serde_json::to_value(&subscription_status)?
        )
        .execute(&self.pool)
        .await?;

        debug!(
            organization_id = organization_id.to_string(),
            subscription_id = ?subscription_status.subscription_id(),
            product = product.to_string(),
            quota_reset = ?quota_reset,
            "Updated subscription information in database"
        );

        Ok(())
    }

    /// If a user creates an organization, it initially gets the [`Role::ReadOnly`] assigned.
    /// This prevents the user from using the API before it is subscribed in Moneybird.
    /// As soon as the organization subscribed for the first time, this user should get admin rights assigned.
    /// As the initial user only had read access to the API, it must be the only user in this organization as inviting other users requires admin access.
    ///
    /// This function elevates the privileges from read-only to admin for this initial user when the organization subscribed for the first time.
    async fn make_user_admin_on_first_subscription(
        &self,
        new_subscription_status: &SubscriptionStatus,
        organization_id: &OrganizationId,
    ) -> Result<(), Error> {
        let old_subscription_json = sqlx::query_scalar!(
            r#"
            SELECT current_subscription AS old_subscription_json
            FROM organizations
            WHERE id = $1
            "#,
            **organization_id,
        )
        .fetch_one(&self.pool)
        .await?;

        let old_subscription_status: SubscriptionStatus =
            serde_json::from_value(old_subscription_json)?;

        if !matches!(
            (&old_subscription_status, &new_subscription_status),
            (&SubscriptionStatus::None, &SubscriptionStatus::Active(_))
        ) {
            trace!(
                old_subscription_status = old_subscription_status.status_string(),
                new_subscription_status = new_subscription_status.status_string(),
                organization_id = organization_id.to_string(),
                "Not updating user privileges",
            );
            return Ok(());
        };

        let api_user_ids = sqlx::query_scalar!(
            r#"
            SELECT id FROM api_users u
                JOIN api_users_organizations o ON u.id = o.api_user_id
            WHERE o.organization_id = $1
            "#,
            **organization_id,
        )
        .fetch_all(&self.pool)
        .await?;

        if api_user_ids.len() != 1 {
            error!(
                organization_id = organization_id.to_string(),
                "Expected exactly one API user for organization but found {} API users",
                api_user_ids.len()
            );
            return Ok(());
        }

        info!(
            old_subscription_status = old_subscription_status.status_string(),
            new_subscription_status = new_subscription_status.status_string(),
            organization_id = organization_id.to_string(),
            "Organization subscribed for the first time. Elevating privileges from read-only to admin",
        );

        sqlx::query!(
            r#"
            UPDATE api_users_organizations
            SET role = 'admin'
            WHERE api_user_id = $1
              AND organization_id = $2
            "#,
            api_user_ids[0],
            **organization_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn calculate_quota_reset_datetime(
        &self,
        subscription: &SubscriptionStatus,
    ) -> Result<Option<DateTime<Utc>>, Error> {
        let date = match subscription {
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
                    Some(*end_date)
                } else {
                    trace!(
                        subscription_id = subscription_id.as_str(),
                        recurring_sales_invoice_id = recurring_sales_invoice_id.as_str(),
                        "Calculating subscription period end based on the next recurring invoice `invoice_date`"
                    );
                    Some(self.api.next_invoice_date(recurring_sales_invoice_id).await?
                        .checked_sub_days(Days::new(1))
                        .ok_or(
                            Error::Moneybird("Could not calculate subscription period end based on the next invoice date".to_string()))?)
                }
            }
            SubscriptionStatus::Expired(_) | SubscriptionStatus::None => {
                trace!("Found no active subscription. Setting quota reset to None");
                None
            }
        };
        Ok(match date {
            None => None,
            Some(date) => Some(
                date.and_hms_opt(23, 59, 59)
                    .ok_or(Error::Moneybird(
                        "Could not add time to subscription end".to_string(),
                    ))?
                    .and_utc(),
            ),
        })
    }

    pub async fn reset_all_quotas(&self) -> Result<(), Error> {
        struct QuotaResetInfo {
            org_id: OrganizationId,
            contact_id: Option<MoneybirdContactId>,
        }
        let quota_infos = sqlx::query_as!(
            QuotaResetInfo,
            r#"
            SELECT id AS org_id,
                   moneybird_contact_id AS "contact_id: MoneybirdContactId"
            FROM organizations
            WHERE quota_reset < now()
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        debug!("resetting quotas for {} organizations", quota_infos.len());

        for quota_info in quota_infos {
            self.reset_single_quota(quota_info.org_id, quota_info.contact_id)
                .await?;
        }

        Ok(())
    }

    async fn reset_single_quota(
        &self,
        organization_id: OrganizationId,
        contact_id: Option<MoneybirdContactId>,
    ) -> Result<(), Error> {
        let subscription_status = if let Some(contact_id) = contact_id {
            self.api
                .get_subscription_status_by_contact_id(&contact_id)
                .await?
        } else {
            SubscriptionStatus::None
        };

        let reset_date = self
            .calculate_quota_reset_datetime(&subscription_status)
            .await?;

        self.store_subscription_status(&subscription_status, &organization_id)
            .await?;

        let quota = subscription_status.quota();

        sqlx::query!(
            r#"
            UPDATE organizations
            SET quota_reset = $2,
                total_message_quota = $3,
                used_message_quota = 0
            WHERE id = $1
            "#,
            *organization_id,
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

        let contact: Contact = self.api.create_contact(&company_name).await?;

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

        self.api.create_sales_link(contact_id).await
    }

    pub async fn refresh_subscription_status(
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

        let status = match contact_id {
            Some(contact_id) => self
                .api
                .get_subscription_status_by_contact_id(&contact_id)
                .await
                .inspect_err(|err| {
                    warn!(
                        organization_id = org_id.as_uuid().to_string(),
                        contact_id = contact_id.as_str(),
                        "Cloud not fetch subscription status from moneybird: {}",
                        err
                    );
                })?,
            None => {
                trace!(
                    organization_id = org_id.as_uuid().to_string(),
                    "No moneybird contact found"
                );
                SubscriptionStatus::None
            }
        };

        self.store_subscription_status(&status, &org_id).await?;

        Ok(status)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::{OrganizationRepository, Role};
    use chrono::{Months, NaiveTime};
    use std::ops::Add;

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

        assert_eq!(active_none, active_none);
        assert!(active_none > active_tomorrow);
        assert!(active_tomorrow < active_none);
        assert!(active_tomorrow > active_today);
        assert!(active_none > active_today);

        assert!(active_today > expired_yesterday);
        assert!(expired_yesterday < active_today);
        assert!(active_none > expired_two_days_ago);

        assert!(expired_yesterday > expired_two_days_ago);

        assert!(active_none > none);
        assert!(none < active_none);
        assert!(active_today > none);
        assert!(active_tomorrow > none);

        assert!(expired_yesterday > none);
        assert!(none < expired_yesterday);
        assert_eq!(none, none);
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn reset_all_quotas(db: PgPool) {
        let moneybird = MoneyBird::new(db.clone()).await.unwrap();

        moneybird.reset_all_quotas().await.unwrap();

        let orgs = OrganizationRepository::new(db.clone())
            .list(None)
            .await
            .unwrap();

        assert_eq!(orgs.len(), 8);
        for org in orgs {
            let (exp_total, reset, exp_subscription_status) =
                match org.id().as_uuid().to_string().as_str() {
                    "44729d9f-a7dc-4226-b412-36a7537f5176" => {
                        (800, Utc::now().checked_add_months(Months::new(1)), "none")
                    }
                    "5d55aec5-136a-407c-952f-5348d4398204" => {
                        (500, Utc::now().checked_add_months(Months::new(1)), "none")
                    }
                    "533d9a19-16e8-4a1b-a824-ff50af8b428c" => (0, None, "none"),
                    "ee14cdb8-f62e-42ac-a0cd-294d708be994" => (0, None, "none"),
                    "7b2d91d0-f9d9-4ddd-88ac-6853f736501c" => (
                        333,
                        Some(Utc::now().add(chrono::Duration::seconds(60))),
                        "none",
                    ),
                    "0f83bfee-e7b6-4670-83ec-192afec2b137" => (0, None, "none"),
                    "ad76a517-3ff2-4d84-8299-742847782d4d" => (
                        1_000,
                        Some(
                            Utc::now()
                                .checked_add_days(Days::new(9))
                                .unwrap()
                                .with_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
                                .unwrap()
                                .to_utc(),
                        ),
                        "active",
                    ),
                    _ => continue,
                };
            assert_eq!(
                org.total_message_quota(),
                exp_total,
                "wrong message quota for {}",
                org.id()
            );

            assert_eq!(
                org.current_subscription().status_string(),
                exp_subscription_status,
                "wrong subscription status for {}",
                org.id()
            );

            match reset {
                None => {
                    assert!(org.quota_reset().is_none())
                }
                Some(exp_reset) => {
                    assert!(
                        (exp_reset.timestamp_millis()
                            - org.quota_reset().unwrap().timestamp_millis())
                        .abs()
                            < 3000,
                        "failed for {}: exp_reset: {:?}, org_reset: {:?}",
                        org.id(),
                        exp_reset,
                        org.quota_reset()
                    )
                }
            }
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    #[tracing_test::traced_test]
    async fn admin_on_first_subscription(db: PgPool) {
        let moneybird = MoneyBird::new(db.clone()).await.unwrap();
        let org_id: OrganizationId = "e11df9da-56f5-433c-9d3a-dd338f262c66".parse().unwrap();

        let new = SubscriptionStatus::Active(Subscription {
            subscription_id: "something".parse().unwrap(),
            product: ProductIdentifier::RmlsFree,
            title: "something".to_string(),
            description: "something".to_string(),
            recurring_sales_invoice_id: "something".parse().unwrap(),
            start_date: Utc::now().date_naive(),
            end_date: None,
            sales_invoices_url: "https://locahost".parse().unwrap(),
        });

        moneybird
            .make_user_admin_on_first_subscription(&new, &org_id)
            .await
            .unwrap();

        assert!(logs_contain(
            "Expected exactly one API user for organization but found 2 API users"
        ));

        let roles = sqlx::query_scalar!(
            r#"
            SELECT role AS "role: Role" FROM api_users_organizations WHERE organization_id = $1
            "#,
            *org_id
        )
        .fetch_all(&db)
        .await
        .unwrap();

        assert_eq!(vec![Role::ReadOnly, Role::ReadOnly], roles);
    }
}
