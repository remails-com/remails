use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct ApiUserId(Uuid);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ApiUserRole {
    SuperAdmin,
    OrganizationAdmin(OrganizationId),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewApiUser {
    pub email: EmailAddress,
    pub roles: Vec<ApiUserRole>,
    pub github_user_id: Option<i64>,
}

pub struct ApiUser {
    id: ApiUserId,
    pub email: String,
    roles: Vec<ApiUserRole>,
    #[allow(unused)]
    github_user_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
impl ApiUser {
    pub fn new(roles: Vec<ApiUserRole>) -> Self {
        Self {
            id: "0b8c948a-8f0c-4b63-a70e-78a9a186f7a2".parse().unwrap(),
            email: "test@test.com".to_string(),
            roles,
            github_user_id: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }
}

impl ApiUser {
    pub fn roles(&self) -> Vec<ApiUserRole> {
        self.roles.clone()
    }
    pub fn id(&self) -> &ApiUserId {
        &self.id
    }
}

struct PgApiUser {
    id: ApiUserId,
    email: String,
    roles: serde_json::Value,
    github_user_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgApiUser> for ApiUser {
    type Error = serde_json::Error;

    fn try_from(u: PgApiUser) -> Result<Self, Self::Error> {
        Ok(Self {
            id: u.id,
            email: u.email,
            roles: serde_json::from_value(u.roles)?,
            github_user_id: u.github_user_id,
            created_at: u.created_at,
            updated_at: u.updated_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ApiUserRepository {
    pool: PgPool,
}

impl ApiUserRepository {
    pub fn new(pool: PgPool) -> Self {
        ApiUserRepository { pool }
    }

    pub async fn create(&self, user: &NewApiUser) -> Result<ApiUser, Error> {
        Ok(sqlx::query_as!(
            PgApiUser,
            r#"
            INSERT INTO api_users (id, email, roles, github_user_id)
            VALUES (gen_random_uuid(), $1, $2, $3)
            RETURNING *
            "#,
            user.email.as_str(),
            serde_json::to_value(&user.roles).unwrap(),
            user.github_user_id
        )
        .fetch_one(&self.pool)
        .await?
        .try_into()?)
    }

    pub async fn find_by_github_id(&self, github_id: i64) -> Result<Option<ApiUser>, Error> {
        Ok(sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT * FROM api_users WHERE github_user_id = $1
            "#,
            github_id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::try_into)
        .transpose()?)
    }

    #[cfg_attr(test, allow(dead_code))]
    pub async fn find_by_id(&self, id: ApiUserId) -> Result<Option<ApiUser>, Error> {
        Ok(sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT * FROM api_users WHERE id = $1
            "#,
            *id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::try_into)
        .transpose()?)
    }
}
