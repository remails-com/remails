use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::message::{Message, MessageFilter, MessageRepository};

use super::{
    auth::ApiUser,
    error::{ApiError, ApiResult},
};

pub async fn list_messages(
    Query(mut filter): Query<MessageFilter>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<Message>> {
    if api_user.is_user() {
        filter.user_id = api_user.get_user_id();
    }

    let messages = repo.list_message_metadata(filter).await.unwrap();

    Ok(Json(messages))
}

pub async fn get_message(
    Path(id): Path<Uuid>,
    State(repo): State<MessageRepository>,
    api_user: ApiUser,
) -> ApiResult<Message> {
    match repo.find_by_id(id).await? {
        Some(message) => {
            if api_user.is_user() && Some(*message.get_id()) != api_user.get_user_id() {
                return Err(ApiError::Forbidden);
            }

            Ok(Json(message))
        }
        None => Err(ApiError::NotFound),
    }
}
