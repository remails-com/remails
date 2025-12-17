use crate::moneybird::{
    Error, MoneybirdApi, Webhook,
    model::{
        AdministrationId, Contact, MoneybirdContactId, ProductIdentifier, RecurringSalesInvoiceId,
        Subscription, SubscriptionId, SubscriptionStatus, SubscriptionTemplate,
        SubscriptionTemplateId, WebhookId,
    },
};
use async_trait::async_trait;
use chrono::{Days, NaiveDate, Utc};
use rand::Rng;
use url::Url;

pub(super) struct MockMoneybirdApi {}

#[async_trait]
impl MoneybirdApi for MockMoneybirdApi {
    async fn register_webhook(&self) -> Option<Webhook> {
        Some(Webhook {
            id: WebhookId("mock_webhook_id".to_string()),
            administration_id: AdministrationId("mock administration".to_string()),
            url: "https://dump.tweede.golf/dump/moneybird".parse().unwrap(),
            token: "supersecuretoken".to_string().into(),
        })
    }

    async fn next_invoice_date(
        &self,
        _recurring_sales_invoice_id: &RecurringSalesInvoiceId,
    ) -> Result<NaiveDate, Error> {
        Ok(Utc::now()
            .date_naive()
            .checked_add_days(Days::new(10))
            .unwrap())
    }

    async fn create_contact(&self, company_name: &str) -> reqwest::Result<Contact> {
        let id = rand::rng().random_range(10000..999999);
        let id = format!("mock_id_{id}").into();

        Ok(Contact {
            id,
            company_name: company_name.to_string(),
            email: "mock@email.com".to_string(),
            phone: "+123456789".to_string(),
            address1: "mock 42".to_string(),
            address2: "".to_string(),
            zipcode: "1234 AB".to_string(),
            city: "Nijmegen".to_string(),
            country: "Netherlands".to_string(),
            sales_invoices_url: "https://tweedegolf.com".parse().unwrap(),
            contact_people: vec![],
        })
    }

    async fn subscription_templates(&self) -> Result<Vec<SubscriptionTemplate>, Error> {
        Ok(vec![SubscriptionTemplate {
            id: SubscriptionTemplateId("mock_subscription_template_id".to_string()),
        }])
    }

    async fn get_subscription_status_by_contact_id(
        &self,
        _contact_id: &MoneybirdContactId,
    ) -> Result<SubscriptionStatus, Error> {
        Ok(SubscriptionStatus::Active(Subscription {
            subscription_id: SubscriptionId("mock_subscription_id".to_string()),
            product: ProductIdentifier::RmlsFree,
            title: "Mock testing subscription".to_string(),
            description: "This is a mock subscription for testing only\nIt uses the same quota as the Remails Free subscription".to_string(),
            recurring_sales_invoice_id: RecurringSalesInvoiceId("mock_invoice_id".to_string()),
            start_date: Utc::now().date_naive().checked_sub_days(Days::new(5)).unwrap(),
            end_date: None,
            sales_invoices_url: "https://tweedegolf.com".parse()?,
        }))
    }

    async fn create_sales_link(
        &self,
        _moneybird_contact_id: MoneybirdContactId,
    ) -> Result<Url, Error> {
        Ok("https://tweedegolf.com".parse()?)
    }

    async fn customer_contact_portal(
        &self,
        _moneybird_contact_id: MoneybirdContactId,
    ) -> Result<Url, Error> {
        Ok("https://tweedegolf.com".parse()?)
    }
}
