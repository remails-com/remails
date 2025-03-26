use crate::models::{ApiUser, Message, MessageFilter, MessageId, MessageRepository};
use axum::{
    Json,
    extract::{Path, Query, State},
};

use super::error::{ApiError, ApiResult};

impl From<ApiUser> for MessageFilter {
    fn from(user: ApiUser) -> Self {
        if user.is_super_admin() {
            Self::default()
        } else {
            let mut filter = Self::default();
            filter.orgs = Some(user.org_admin());
            filter
        }
    }
}

pub async fn list_messages(
    Query(mut filter): Query<MessageFilter>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<Message>> {
    if !api_user.is_super_admin() {
        if let Some(filter_orgs) = filter.orgs {
            filter.orgs = Some(
                api_user
                    .org_admin()
                    .into_iter()
                    .filter(|user_org| filter_orgs.contains(user_org))
                    .collect(),
            );
        } else {
            filter.orgs = Some(api_user.org_admin())
        }
    }

    let messages = repo.list_message_metadata(filter).await?;

    Ok(Json(messages))
}

pub async fn get_message(
    Path(id): Path<MessageId>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Message> {
    let filter: MessageFilter = api_user.into();

    let message = repo
        .find_by_id(id, filter)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(message))
}
