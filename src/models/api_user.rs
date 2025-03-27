use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, From, Display, Deref, FromStr)]
pub struct ApiUserId(Uuid);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(PartialOrd, Ord, Eq))]
pub enum ApiUserRole {
    SuperAdmin,
    OrganizationAdmin(OrganizationId),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewApiUser {
    pub email: EmailAddress,
    pub roles: Vec<ApiUserRole>,
    pub github_user_id: Option<i64>,
}

#[derive(Debug)]
pub struct ApiUser {
    id: ApiUserId,
    pub email: EmailAddress,
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
            email: "test@test.com".parse().unwrap(),
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
    email: String,
    organization_roles: Vec<PgOrgRole>,
    global_roles: Vec<Option<PgRole>>,
    github_user_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PgApiUser> for ApiUser {
    type Error = Error;

    fn try_from(u: PgApiUser) -> Result<Self, Self::Error> {
        let mut roles: Vec<ApiUserRole> = u
            .organization_roles
            .into_iter()
            .filter_map(|role| {
                role.role.zip(role.org_id).map(|(role, org_id)| match role {
                    PgRole::Admin => ApiUserRole::OrganizationAdmin(org_id),
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
            email: u.email.parse()?,
            roles,
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

    pub async fn create(&self, user: NewApiUser) -> Result<ApiUser, Error> {
        let mut tx = self.pool.begin().await?;

        let user_id = sqlx::query_scalar!(
            r#"
            INSERT INTO api_users (id, email, github_user_id)
            VALUES (gen_random_uuid(), $1, $2)
            RETURNING id
            "#,
            user.email.as_str(),
            user.github_user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let (organization_roles, global_roles) = user.roles.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut orgs, mut global), role| {
                match role {
                    ApiUserRole::SuperAdmin => global.push(PgRole::Admin),
                    ApiUserRole::OrganizationAdmin(org) => orgs.push((org, PgRole::Admin)),
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
                INSERT INTO api_users_global_role (api_user_id, role) 
                VALUES ($1, $2)
                "#,
                user_id,
                global_role as PgRole
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Ok(self.find_by_id(user_id.into()).await?.unwrap())
    }

    pub async fn find_by_github_id(&self, github_id: i64) -> Result<Option<ApiUser>, Error> {
        sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT u.*,
                   array_agg((o.organization_id,o.role)::org_role)::org_role[] AS "organization_roles!: Vec<PgOrgRole>",
                   array_agg(distinct g.role) AS "global_roles!: Vec<Option<PgRole>>"
            FROM api_users u 
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
                LEFT JOIN api_users_global_role g ON u.id = g.api_user_id 
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

    #[cfg_attr(test, allow(dead_code))]
    pub async fn find_by_id(&self, id: ApiUserId) -> Result<Option<ApiUser>, Error> {
        sqlx::query_as!(
            PgApiUser,
            r#"
            SELECT u.*,
                   array_agg((o.organization_id,o.role)::org_role)::org_role[] AS "organization_roles!: Vec<PgOrgRole>",
                   array_agg(distinct g.role) AS "global_roles!: Vec<Option<PgRole>>"
            FROM api_users u 
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
                LEFT JOIN api_users_global_role g ON u.id = g.api_user_id 
            WHERE u.id = $1
            GROUP BY u.id
            "#,
            *id
        )
            .fetch_optional(&self.pool)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
}

#[cfg(test)]
mod test {
    use crate::models::{ApiUser, ApiUserRepository, ApiUserRole, NewApiUser};
    use sqlx::PgPool;

    impl PartialEq<NewApiUser> for ApiUser {
        fn eq(&self, other: &NewApiUser) -> bool {
            let mut other_roles = other.roles.clone();
            other_roles.sort();

            let mut self_roles = self.roles.clone();
            self_roles.sort();

            self.github_user_id == other.github_user_id
                && self.email == other.email
                && self_roles == other_roles
        }
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations")))]
    async fn create_user(db: PgPool) {
        let repo = ApiUserRepository::new(db);

        let user = NewApiUser {
            email: "test@email.com".parse().unwrap(),
            roles: vec![
                ApiUserRole::SuperAdmin,
                ApiUserRole::OrganizationAdmin(
                    "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
                ),
            ],
            github_user_id: Some(123),
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);

        let user = NewApiUser {
            email: "test2@email.com".parse().unwrap(),
            roles: vec![],
            github_user_id: None,
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);
    }
}
