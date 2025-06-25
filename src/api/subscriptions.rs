use crate::{
    api::{
        error::{ApiError, ApiResult},
        ApiState,
    },
    models::{ApiUser, OrganizationId},
    moneybird::{Error, MoneyBird, MoneybirdWebhookPayload, SubscriptionStatus},
};
use axum::{
    debug_handler, extract::{Path, State},
    Json,
};
use tracing::debug;
use url::Url;

fn has_read_access(org: OrganizationId, user: &ApiUser) -> Result<(), ApiError> {
    if user.org_admin().contains(&org) || user.is_super_admin() {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn get_subscription(
    State(moneybird): State<MoneyBird>,
    user: ApiUser,
    Path(org): Path<OrganizationId>,
) -> ApiResult<SubscriptionStatus> {
    has_read_access(org, &user)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        "get subscription"
    );

    Ok(Json(moneybird.get_subscription_status(org).await?))
}

pub async fn get_sales_link(
    State(moneybird): State<MoneyBird>,
    user: ApiUser,
    Path(org): Path<OrganizationId>,
) -> ApiResult<Url> {
    has_read_access(org, &user)?;

    debug!(
        user_id = user.id().to_string(),
        organization_id = org.to_string(),
        "create sales link"
    );

    Ok(Json(moneybird.create_sales_link(org).await?))
}

// TODO authentication?
pub async fn moneybird_webhook(
    State(moneybird): State<MoneyBird>,
    Json(payload): Json<MoneybirdWebhookPayload>,
) -> Result<(), ApiError> {
    debug!("received moneybird webhook");
    moneybird.webhook_handler(payload).await?;
    Ok(())
}
