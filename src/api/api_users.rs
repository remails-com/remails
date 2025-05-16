use crate::{
    api::{
        error::{ApiError, ApiResult},
        whoami::WhoamiResponse,
    },
    models::{
        ApiUser, ApiUserId, ApiUserRepository, ApiUserUpdate, Error, Password, PasswordUpdate,
    },
};
use axum::{
    Json,
    extract::{Path, State},
};

fn has_write_access(user_id: ApiUserId, user: &ApiUser) -> Result<(), ApiError> {
    if *user.id() == user_id {
        return Ok(());
    }
    Err(ApiError::Forbidden)
}

pub async fn update_user(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<ApiUserUpdate>,
) -> ApiResult<WhoamiResponse> {
    has_write_access(user_id, &user)?;

    repo.update(update, &user_id).await?;

    Ok(Json(
        repo.find_by_id(&user_id)
            .await
            .transpose()
            .ok_or(Error::NotFound("User not found"))??
            .into(),
    ))
}

pub async fn update_password(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<PasswordUpdate>,
) -> Result<(), ApiError> {
    has_write_access(user_id, &user)?;

    repo.update_password(update, &user_id).await?;

    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentPassword {
    current_password: Password,
}

pub async fn delete_password(
    State(repo): State<ApiUserRepository>,
    Path(user_id): Path<ApiUserId>,
    user: ApiUser,
    Json(update): Json<CurrentPassword>,
) -> Result<(), ApiError> {
    has_write_access(user_id, &user)?;

    if user.github_user_id().is_none() {
        Err(ApiError::PreconditionFailed(
            "You must enable an alternative login method before you can delete your password"
                .to_string(),
        ))?
    }

    repo.delete_password(update.current_password, &user_id)
        .await?;

    Ok(())
}
