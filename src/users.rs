use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct User {
    id: Uuid,
    username: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    #[cfg(test)]
    pub(crate) fn new(username: String, password: String) -> Self {
        let password_hash = password_auth::generate_hash(password.as_bytes());

        Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub(crate) fn verify_password(&self, password: &str) -> bool {
        password_auth::verify_password(password.as_bytes(), &self.password_hash).is_ok()
    }

    pub(crate) fn get_id(&self) -> Uuid {
        self.id
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UserRepository {
    pool: sqlx::PgPool,
}

impl UserRepository {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[cfg(test)]
    pub(crate) async fn insert(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            user.id,
            user.username,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>, sqlx::Error> {
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
