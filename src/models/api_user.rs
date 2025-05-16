use crate::models::{Error, OrganizationId};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct ApiUserId(Uuid);

#[derive(From, derive_more::Debug, Deserialize)]
#[debug("*****")]
#[serde(transparent)]
pub struct Password(String);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
#[cfg_attr(test, derive(PartialOrd, Ord, Eq))]
pub enum ApiUserRole {
    SuperAdmin,
    OrganizationAdmin { id: OrganizationId },
}

#[derive(Debug)]
pub struct NewApiUser {
    pub email: EmailAddress,
    pub name: String,
    pub password: Option<Password>,
    pub roles: Vec<ApiUserRole>,
    pub github_user_id: Option<i64>,
}

#[derive(Debug)]
pub struct ApiUser {
    id: ApiUserId,
    pub name: String,
    pub email: EmailAddress,
    roles: Vec<ApiUserRole>,
    #[allow(unused)]
    github_user_id: Option<i64>,
    password_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApiUserUpdate {
    pub name: String,
    pub email: EmailAddress,
}

#[derive(Debug, Deserialize)]
pub struct PasswordUpdate {
    pub new_password: Password,
    pub current_password: Password,
}

impl ApiUser {
    pub fn roles(&self) -> Vec<ApiUserRole> {
        self.roles.clone()
    }
    pub fn id(&self) -> &ApiUserId {
        &self.id
    }
    pub fn github_user_id(&self) -> Option<i64> {
        self.github_user_id
    }
    pub fn password_enabled(&self) -> bool {
        self.password_enabled
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "role", rename_all = "lowercase")]
enum PgRole {
    Admin,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "org_role")]
struct PgOrgRole {
    org_id: Option<OrganizationId>,
    role: Option<PgRole>,
}

struct PgApiUser {
    id: ApiUserId,
    name: String,
    email: String,
    organization_roles: Vec<PgOrgRole>,
    global_roles: Vec<Option<PgRole>>,
    github_user_id: Option<i64>,
    password_enabled: bool,
}

impl TryFrom<PgApiUser> for ApiUser {
    type Error = Error;

    fn try_from(u: PgApiUser) -> Result<Self, Self::Error> {
        let mut roles: Vec<ApiUserRole> = u
            .organization_roles
            .into_iter()
            .filter_map(|role| {
                role.role.zip(role.org_id).map(|(role, org_id)| match role {
                    PgRole::Admin => ApiUserRole::OrganizationAdmin { id: org_id },
                })
            })
            .collect();
        roles.append(
            &mut u
                .global_roles
                .into_iter()
                .filter_map(|role| {
                    role.map(|r| match r {
                        PgRole::Admin => ApiUserRole::SuperAdmin,
                    })
                })
                .collect(),
        );
        Ok(Self {
            id: u.id,
            name: u.name,
            email: u.email.parse()?,
            roles,
            github_user_id: u.github_user_id,
            password_enabled: u.password_enabled,
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

    pub async fn create(&self, user: NewApiUser) -> Result<ApiUser, Error> {
        let password_hash = user.password.map(|pw| password_auth::generate_hash(pw.0));

        let mut tx = self.pool.begin().await?;

        let user_id = sqlx::query_scalar!(
            r#"
            INSERT INTO api_users (id, email, name, password_hash, github_user_id)
            VALUES (gen_random_uuid(), $1, $2, $3, $4)
            RETURNING id
            "#,
            user.email.as_str(),
            user.name,
            password_hash,
            user.github_user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let (organization_roles, global_roles) = user.roles.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut orgs, mut global), role| {
                match role {
                    ApiUserRole::SuperAdmin => global.push(PgRole::Admin),
                    ApiUserRole::OrganizationAdmin { id: org } => orgs.push((org, PgRole::Admin)),
                }
                (orgs, global)
            },
        );

        for org_role in organization_roles {
            sqlx::query!(
                r#"
                INSERT INTO api_users_organizations (api_user_id, organization_id, role) 
                VALUES ($1, $2, $3)
                "#,
                user_id,
                *org_role.0,
                org_role.1 as PgRole
            )
            .execute(&mut *tx)
            .await?;
        }

        for global_role in global_roles {
            sqlx::query!(
                r#"
                INSERT INTO api_users_global_roles (api_user_id, role) 
                VALUES ($1, $2)
                "#,
                user_id,
                global_role as PgRole
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Ok(self.find_by_id(&user_id.into()).await?.unwrap())
    }

    pub async fn find_by_github_id(&self, github_id: i64) -> Result<Option<ApiUser>, Error> {
        sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT u.id,
                   u.email,
                   u.name,
                   u.github_user_id,
                   array_agg((o.organization_id,o.role)::org_role)::org_role[] AS "organization_roles!: Vec<PgOrgRole>",
                   array_agg(distinct g.role) AS "global_roles!: Vec<Option<PgRole>>",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u 
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
                LEFT JOIN api_users_global_roles g ON u.id = g.api_user_id 
            WHERE github_user_id = $1
            GROUP BY u.id
            "#,
            github_id
        )
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    pub async fn add_github_id(&self, user_id: &ApiUserId, github_id: i64) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE api_users SET github_user_id = $2 WHERE id = $1 
            "#,
            **user_id,
            github_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_github_id(&self, api_user_id: &ApiUserId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE api_users SET github_user_id = NULL WHERE id = $1 
            "#,
            **api_user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, update: ApiUserUpdate, user_id: &ApiUserId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE api_users SET name = $2, email = $3 WHERE id = $1 
            "#,
            **user_id,
            update.name,
            update.email.as_str(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_password(
        &self,
        update: PasswordUpdate,
        user_id: &ApiUserId,
    ) -> Result<(), Error> {
        let hash = sqlx::query_scalar!(
            r#"
            SELECT password_hash FROM api_users WHERE id = $1
            "#,
            **user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(hash) = hash {
            password_auth::verify_password(update.current_password.0, &hash)
                .inspect_err(|err| {
                    tracing::trace!(user_id = user_id.to_string(), "wrong password: {}", err)
                })
                .map_err(|_| Error::BadRequest("wrong password".to_string()))?;
        }

        let hash = password_auth::generate_hash(update.new_password.0);
        sqlx::query!(
            r#"
            UPDATE api_users SET password_hash = $2 WHERE id = $1 
            "#,
            **user_id,
            hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_password(
        &self,
        current_password: Password,
        user_id: &ApiUserId,
    ) -> Result<(), Error> {
        let hash = sqlx::query_scalar!(
            r#"
            SELECT password_hash FROM api_users WHERE id = $1
            "#,
            **user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(hash) = hash {
            password_auth::verify_password(current_password.0, &hash)
                .inspect_err(|err| {
                    tracing::trace!(user_id = user_id.to_string(), "wrong password: {}", err)
                })
                .map_err(|_| Error::BadRequest("wrong password".to_string()))?;
        } else {
            return Ok(());
        };

        sqlx::query!(
            r#"
            UPDATE api_users SET password_hash = NULL WHERE id = $1 
            "#,
            **user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    pub async fn find_by_id(&self, id: &ApiUserId) -> Result<Option<ApiUser>, Error> {
        sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT u.id,
                   u.email,
                   u.name,
                   u.github_user_id,
                   array_agg((o.organization_id,o.role)::org_role)::org_role[] AS "organization_roles!: Vec<PgOrgRole>",
                   array_agg(distinct g.role) AS "global_roles!: Vec<Option<PgRole>>",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u 
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
                LEFT JOIN api_users_global_roles g ON u.id = g.api_user_id 
            WHERE u.id = $1
            GROUP BY u.id
            "#,
            **id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::try_into)
        .transpose()
    }

    pub async fn find_by_email(&self, email: &EmailAddress) -> Result<Option<ApiUser>, Error> {
        sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT u.id,
                   u.email,
                   u.name,
                   u.github_user_id,
                   array_agg((o.organization_id,o.role)::org_role)::org_role[] AS "organization_roles!: Vec<PgOrgRole>",
                   array_agg(distinct g.role) AS "global_roles!: Vec<Option<PgRole>>",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
                LEFT JOIN api_users_global_roles g ON u.id = g.api_user_id
            WHERE u.email = $1
            GROUP BY u.id
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::try_into)
        .transpose()
    }

    pub async fn check_password(
        &self,
        email: &EmailAddress,
        password: Password,
    ) -> Result<(), Error> {
        let hash = sqlx::query_scalar!(
            r#"
            SELECT password_hash FROM api_users WHERE email = $1
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        if let Some(hash) = hash {
            password_auth::verify_password(password.0, &hash)
                .inspect_err(|err| tracing::trace!("wrong password for {}: {}", email, err))
                .map_err(|_| Error::NotFound("User not found or wrong password"))?;
            return Ok(());
        }
        Err(Error::NotFound("User not found or wrong password"))
    }
}

#[cfg(test)]
mod test {
    use crate::models::{ApiUser, ApiUserRepository, ApiUserRole, NewApiUser};
    use sqlx::PgPool;

    impl ApiUser {
        pub fn new(roles: Vec<ApiUserRole>) -> Self {
            Self {
                id: "0b8c948a-8f0c-4b63-a70e-78a9a186f7a2".parse().unwrap(),
                name: "Test Api User".to_string(),
                email: "test@test.com".parse().unwrap(),
                roles,
                github_user_id: None,
                password_enabled: false,
            }
        }
    }

    impl PartialEq<NewApiUser> for ApiUser {
        fn eq(&self, other: &NewApiUser) -> bool {
            let mut other_roles = other.roles.clone();
            other_roles.sort();

            let mut self_roles = self.roles.clone();
            self_roles.sort();

            self.github_user_id == other.github_user_id
                && self.email == other.email
                && self.name == other.name
                && self_roles == other_roles
        }
    }

    impl Clone for NewApiUser {
        fn clone(&self) -> Self {
            Self {
                email: self.email.clone(),
                name: self.name.clone(),
                password: None,
                roles: self.roles.clone(),
                github_user_id: self.github_user_id,
            }
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn create_user(db: PgPool) {
        let repo = ApiUserRepository::new(db);

        let user = NewApiUser {
            email: "test@email.com".parse().unwrap(),
            name: "Test User".to_string(),
            password: None,
            roles: vec![
                ApiUserRole::SuperAdmin,
                ApiUserRole::OrganizationAdmin {
                    id: "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                },
            ],
            github_user_id: Some(123),
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);

        let user = NewApiUser {
            email: "test2@email.com".parse().unwrap(),
            name: "Test User 2".to_string(),
            password: None,
            roles: vec![],
            github_user_id: None,
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);
    }
}
