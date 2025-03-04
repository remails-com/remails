use axum::{Json, extract::State};
use serde::Deserialize;

use crate::user::{User, UserRepository};

use super::{
    auth::ApiUser,
    error::{ApiError, ApiResult},
};

#[derive(Debug, Deserialize)]
pub struct NewUser {
    username: String,
    password: String,
}

pub async fn create_user(
    State(repo): State<UserRepository>,
    api_user: ApiUser,
    Json(NewUser { username, password }): Json<NewUser>,
) -> ApiResult<User> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let new_user = User::new(username, password);
    let user = repo.insert(&new_user).await?;

    Ok(Json(user))
}

pub async fn list_users(
    State(repo): State<UserRepository>,
    api_user: ApiUser,
) -> ApiResult<Vec<User>> {
    if !api_user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let users = repo.list().await?;

    Ok(Json(users))
}
