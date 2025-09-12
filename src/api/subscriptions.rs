use crate::{
    api::error::{ApiError, ApiResult},
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

    Ok(Json(moneybird.get_subscription_status(org_id).await?))
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
