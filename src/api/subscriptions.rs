use crate::{
    api::{
        auth::Authenticated,
        error::{ApiError, ApiResult},
    },
    models::{ApiUser, OrganizationId},
    moneybird::{MoneyBird, MoneybirdWebhookPayload, SubscriptionStatus},
};
use axum::{
    Json,
    extract::{Path, State},
};
use tracing::debug;
use url::Url;

pub async fn get_subscription(
    State(moneybird): State<MoneyBird>,
    user: ApiUser,
    Path(org_id): Path<OrganizationId>,
) -> ApiResult<SubscriptionStatus> {
    user.has_org_read_access(&org_id)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "get subscription"
    );

    Ok(Json(moneybird.refresh_subscription_status(org_id).await?))
}

pub async fn get_sales_link(
    State(moneybird): State<MoneyBird>,
    user: ApiUser,
    Path(org_id): Path<OrganizationId>,
) -> ApiResult<Url> {
    user.has_org_read_access(&org_id)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org_id.to_string(),
        "create sales link"
    );

    Ok(Json(moneybird.create_sales_link(org_id).await?))
}

pub async fn moneybird_webhook(
    State(moneybird): State<MoneyBird>,
    Json(payload): Json<MoneybirdWebhookPayload>,
) -> Result<(), ApiError> {
    debug!("received moneybird webhook");
    moneybird.webhook_handler(payload).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        ProductIdentifier, SubscriptionStatus,
        api::{
            tests::{TestServer, deserialize_body, serialize_body},
            whoami::WhoamiResponse,
        },
        models::{OrgRole, Organization, Role},
    };
    use chrono::Utc;
    use http::StatusCode;
    use serde_json::json;
    use sqlx::PgPool;
    use std::time::Duration;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn webhook(db: PgPool) {
        let server = TestServer::new(
            db.clone(),
            Some("d57373be-cb77-4a2b-9e6e-66b28c4b5c7e".parse().unwrap()),
        )
        .await;

        // Wait for the webhook to be registered
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Send webhook. Important are: webhook_id, webhook_token, product.identifier, and contact.id
        let response = server
            .post(
                "/api/webhook/moneybird",
                serialize_body(json!({
                    "administration_id": "mock_admin_id",
                    "webhook_id": "mock_webhook_id",
                    "webhook_token": "supersecuretoken",
                    "entity_type": "Subscription",
                    "action": "subscription_created",
                    "entity": {
                        "id": "mock_subscription_id",
                        "administration_id": "mock_admin_id",
                        "start_date": Utc::now().date_naive(),
                        "end_date": serde_json::Value::Null,
                        "product": {
                            "id": "mock_product_id",
                            "administration_id": "mock_admin_id",
                            "description": "Webhook test product",
                            "title": "Webhook test",
                            "identifier": "RMLS-FREE"
                        },
                        "contact": {
                            "id": "webhook_test_org",
                            "company_name": "mock company B.V.",
                            "email": "mock_email@company.com",
                            "phone": "+1234567",
                            "address1": "mock_address1",
                            "address2": "mock_address2",
                            "zipcode": "1234AB",
                            "city": "Nijmegen",
                            "country": "NL",
                            "sales_invoices_url": "https://tweedegolf.com",
                            "contact_people": []
                        },
                        "recurring_sales_invoice_id": "mock_recurring_sales_invoice_id"
                    },
                })),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // check that the subscription got updated in the database
        let response = server
            .get("/api/organizations/ad76a517-3ff2-4d84-8299-742847782d4d")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let org: Organization = deserialize_body(response.into_body()).await;
        assert_eq!(
            org.id(),
            "ad76a517-3ff2-4d84-8299-742847782d4d".parse().unwrap()
        );
        assert_eq!(
            org.total_message_quota(),
            ProductIdentifier::RmlsFree.monthly_quota() as i64
        );

        let SubscriptionStatus::Active(subscription) = org.current_subscription() else {
            panic!("No active subscription found in Organization");
        };
        assert_eq!(*subscription.id(), "mock_subscription_id".parse().unwrap());
        assert!(matches!(
            subscription.product_id(),
            ProductIdentifier::RmlsFree
        ));

        // check the user is admin of that organization now
        let response = server.get("/api/whoami").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let whoami: WhoamiResponse = deserialize_body(response.into_body()).await;
        let whoami = whoami.unwrap_logged_in();
        assert_eq!(whoami.org_roles.len(), 1);
        assert_eq!(
            whoami.org_roles[0],
            OrgRole {
                role: Role::Admin,
                org_id: "ad76a517-3ff2-4d84-8299-742847782d4d".parse().unwrap(),
            }
        );
    }
}
