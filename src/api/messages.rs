use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

use crate::models::{Message, MessageFilter, MessageRepository};

use super::{
    auth::ApiUser,
    error::{ApiError, ApiResult},
};

impl TryFrom<ApiUser> for MessageFilter {
    type Error = ApiError;

    fn try_from(user: ApiUser) -> Result<Self, Self::Error> {
        if user.is_admin() {
            Ok(Self::default())
        } else if let Some(user_id) = user.user_id() {
            let mut filter = Self::default();
            filter.api_user_id = Some(user_id);
            Ok(filter)
        } else {
            Err(ApiError::Unauthorized)
        }
    }
}

pub async fn list_messages(
    Query(mut filter): Query<MessageFilter>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<Message>> {
    if api_user.is_user() {
        filter.api_user_id = api_user.user_id();
    }

    let messages = repo.list_message_metadata(filter).await?;

    Ok(Json(messages))
}

pub async fn get_message(
    Path(id): Path<Uuid>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Message> {
    let filter: MessageFilter = api_user.try_into()?;

    let message = repo
        .find_by_id(id, filter.api_user_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(message))
}
