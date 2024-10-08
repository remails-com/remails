use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    id: Uuid,
    username: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, password: String) -> Self {
        let password_hash = password_auth::generate_hash(password.as_bytes());

        Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: sqlx::PgPool,
}

impl UserRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, new_user: &User) -> Result<User, sqlx::Error> {
        let user: User = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, username, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            new_user.id,
            new_user.username,
            new_user.password_hash,
            new_user.created_at,
            new_user.updated_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT * FROM users WHERE username = $1 LIMIT 1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn list(&self) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT * FROM users ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn user_repository(pool: PgPool) {
        let repository = UserRepository::new(pool);

        let user = User::new("test".into(), "password".into());
        assert!(user.verify_password("password"));

        repository.insert(&user).await.unwrap();

        let fetched_user = repository.find_by_username("test").await.unwrap().unwrap();

        assert_eq!(fetched_user.username, "test");
        assert!(fetched_user.verify_password("password"));
    }
}
