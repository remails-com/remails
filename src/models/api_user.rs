use crate::models::{Error, OrganizationId};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use email_address::EmailAddress;
use garde::Validate;
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::trace;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    FromStr,
    ToSchema,
    IntoParams,
)]
#[into_params(names("user_id"))]
pub struct ApiUserId(Uuid);

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    FromStr,
    ToSchema,
    IntoParams,
)]
#[into_params(names("totp_id"))]
pub struct TotpId(Uuid);

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    From,
    Display,
    Deref,
    FromStr,
    ToSchema,
    IntoParams,
)]
#[into_params(names("pw_reset_id"))]
pub struct PwResetId(Uuid);

#[derive(From, derive_more::Debug, Deserialize, FromStr, ToSchema, Validate)]
#[debug("*****")]
#[serde(transparent)]
#[schema(format = Password)]
pub struct Password(#[garde(length(min = 6, max = 256))] String);

impl Password {
    pub fn generate_hash(&self) -> String {
        password_auth::generate_hash(self.0.as_str())
    }

    pub fn verify_password(&self, hash: &str) -> Result<(), password_auth::VerifyError> {
        password_auth::verify_password(&self.0, hash)
    }

    pub fn new(password: String) -> Self {
        Password(password)
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    Display,
    PartialEq,
    PartialOrd,
    sqlx::Type,
    ToSchema,
    Validate,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "role", rename_all = "snake_case")]
#[cfg_attr(test, derive(Ord, Eq))]
pub enum Role {
    ReadOnly = 0,
    Maintainer = 1,
    Admin = 2,
}

impl Role {
    /// Check if role is at least a certain level
    ///
    /// Read-only < Maintainer < Admin
    pub fn is_at_least(&self, role: Role) -> bool {
        *self >= role
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[cfg_attr(test, derive(PartialEq, PartialOrd, Ord, Eq))]
#[serde(rename_all = "snake_case")]
pub struct OrgRole {
    pub role: Role,
    pub org_id: OrganizationId,
}

#[derive(Debug)]
pub struct NewApiUser {
    pub email: EmailAddress,
    pub name: String,
    pub password: Option<Password>,
    pub global_role: Option<Role>,
    pub org_roles: Vec<OrgRole>,
    pub github_user_id: Option<i64>,
}

#[derive(Debug)]
pub struct ApiUser {
    id: ApiUserId,
    pub name: String,
    /// Logged-in session users always have an email, but API keys do not
    pub email: Option<EmailAddress>,
    pub global_role: Option<Role>,
    pub org_roles: Vec<OrgRole>,
    pub github_user_id: Option<i64>,
    pub password_enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[cfg_attr(test, derive(Serialize))]
pub struct ApiUserUpdate {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(skip)]
    pub email: EmailAddress,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct PasswordUpdate {
    #[garde(dive)]
    #[schema(min_length = 6, max_length = 256)]
    pub new_password: Password,
    #[garde(dive)]
    #[schema(min_length = 6, max_length = 256)]
    pub current_password: Option<Password>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpFinishEnroll {
    #[garde(dive)]
    code: TotpCode,
    #[schema(max_length = 500)]
    #[garde(length(max = 500))]
    #[serde(default)]
    description: String,
}

#[derive(Deserialize, Validate, Debug, ToSchema)]
#[serde(transparent)]
#[schema(pattern = "^[0-9]{6}$")]
pub struct TotpCode(#[garde(pattern("^[0-9]{6}$"))] String);

impl AsRef<str> for TotpCode {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(Deserialize))]
pub struct TotpCodeDetails {
    pub id: TotpId,
    pub description: String,
    pub last_used: Option<DateTime<Utc>>,
}

impl TotpCodeDetails {
    pub fn id(&self) -> &TotpId {
        &self.id
    }
}

impl ApiUser {
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

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "org_role")]
struct PgOrgRole {
    org_id: Option<OrganizationId>,
    role: Option<Role>,
}

impl From<PgOrgRole> for Option<OrgRole> {
    fn from(role: PgOrgRole) -> Self {
        Some(OrgRole {
            org_id: role.org_id?,
            role: role.role?,
        })
    }
}

struct PgApiUser {
    id: ApiUserId,
    name: String,
    email: String,
    organization_roles: Vec<PgOrgRole>,
    global_role: Option<Role>,
    github_user_id: Option<i64>,
    password_enabled: bool,
}

impl TryFrom<PgApiUser> for ApiUser {
    type Error = Error;

    fn try_from(u: PgApiUser) -> Result<Self, Self::Error> {
        let org_roles = u
            .organization_roles
            .into_iter()
            .filter_map(|role| role.into())
            .collect();
        Ok(Self {
            id: u.id,
            name: u.name,
            email: Some(u.email.parse()?),
            global_role: u.global_role,
            org_roles,
            github_user_id: u.github_user_id,
            password_enabled: u.password_enabled,
        })
    }
}

/// Indicates weather a password reset link is still valid and weather or not 2FA is activated on that account
#[derive(Serialize, ToSchema, Debug)]
#[cfg_attr(test, derive(Deserialize, PartialEq, Eq))]
pub enum ResetLinkCheck {
    NotActive,
    ActiveWithout2Fa,
    ActiveWith2Fa,
}

#[derive(derive_more::Debug)]
pub struct PwResetData {
    pub pw_reset_id: PwResetId,
    #[debug("*******")]
    pub reset_secret: String,
    pub user_name: String,
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
        let password_hash = user.password.map(|pw| pw.generate_hash());

        let mut tx = self.pool.begin().await?;

        let user_id = sqlx::query_scalar!(
            r#"
            INSERT INTO api_users (id, email, name, password_hash, github_user_id, global_role)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)
            RETURNING id
            "#,
            user.email.as_str(),
            user.name,
            password_hash,
            user.github_user_id,
            user.global_role as Option<Role>
        )
        .fetch_one(&mut *tx)
        .await?;

        for OrgRole { role, org_id } in user.org_roles {
            sqlx::query!(
                r#"
                INSERT INTO api_users_organizations (api_user_id, organization_id, role)
                VALUES ($1, $2, $3)
                "#,
                user_id,
                *org_id,
                role as Role
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(self.find_by_id(&user_id.into()).await?.unwrap())
    }

    pub async fn start_enroll_totp(&self, user_id: &ApiUserId) -> Result<Vec<u8>, Error> {
        let mut tx = self.pool.begin().await?;

        let email = sqlx::query_scalar!(r#"SELECT email FROM api_users WHERE id = $1"#, **user_id)
            .fetch_one(&mut *tx)
            .await?;

        // Make sure there is only one TOTP token enrolling at a time
        sqlx::query!(
            r#"
            DELETE FROM totp WHERE user_id = $1 AND state = 'enrolling';
            "#,
            **user_id
        )
        .execute(&mut *tx)
        .await?;

        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1,
            30,
            Secret::generate_secret().to_bytes().unwrap(),
            Some("Remails".to_string()),
            email,
        )?;

        sqlx::query!(
            r#"
            INSERT INTO totp (id, description, user_id, url)
            VALUES (gen_random_uuid(), 'Not yet activated' , $1, $2)
            "#,
            **user_id,
            totp.get_url()
        )
        .execute(&mut *tx)
        .await?;

        let png = totp.get_qr_png().map_err(Error::Internal);
        tx.commit().await?;
        png
    }

    pub async fn finish_enroll_totp(
        &self,
        user_id: &ApiUserId,
        finish: TotpFinishEnroll,
    ) -> Result<TotpCodeDetails, Error> {
        let mut tx = self.pool.begin().await?;

        let url = sqlx::query_scalar!(
            r#"
            SELECT url FROM totp WHERE user_id = $1 AND state = 'enrolling'
            "#,
            **user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let totp = TOTP::from_url(url)?;

        if !totp.check(finish.code.as_ref(), Utc::now().timestamp() as u64) {
            return Err(Error::BadRequest("Invalid TOTP code".to_string()));
        }

        let code = sqlx::query_as!(
            TotpCodeDetails,
            r#"
            UPDATE totp SET state = 'enabled',
                            description = $2
            WHERE user_id = $1
              AND state = 'enrolling'
            RETURNING
                id,
                description,
                last_used
            "#,
            **user_id,
            finish.description
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(code)
    }

    pub async fn mfa_enabled(&self, user_id: &ApiUserId) -> Result<bool, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT EXISTS(SELECT 1 FROM totp WHERE user_id = $1 AND state = 'enabled') as "exists!"
            "#,
            **user_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn check_totp_code(&self, user_id: &ApiUserId, code: &str) -> Result<bool, Error> {
        self.check_and_increase_totp_try_counter(user_id).await?;

        struct Totp {
            id: TotpId,
            url: String,
        }

        let totps = sqlx::query_as!(
            Totp,
            r#"
            SELECT id, url FROM totp WHERE user_id = $1 AND state = 'enabled'
            "#,
            **user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let now = Utc::now().timestamp() as u64;

        for Totp { id, url } in totps {
            let totp = TOTP::from_url(url)?;

            if totp.check(code, now) {
                sqlx::query!(
                    "
                    UPDATE totp SET last_used = now() where id = $1
                    ",
                    *id
                )
                .execute(&self.pool)
                .await?;

                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn check_and_increase_totp_try_counter(&self, user_id: &ApiUserId) -> Result<(), Error> {
        let counter = sqlx::query_scalar!(
            r#"
            UPDATE api_users
            SET totp_try_counter       = CASE
                                             WHEN totp_try_counter_reset < now() THEN 0
                                             ELSE totp_try_counter + 1 END,
                totp_try_counter_reset = CASE
                                             WHEN totp_try_counter_reset < now() THEN now() + '1 min'
                                             ELSE totp_try_counter_reset END
            WHERE id = $1
            RETURNING totp_try_counter;
            "#,
            **user_id
        )
            .fetch_one(&self.pool).await?;

        if counter > 3 {
            Err(Error::TooManyRequests)
        } else {
            Ok(())
        }
    }

    pub async fn totp_codes(&self, user_id: &ApiUserId) -> Result<Vec<TotpCodeDetails>, Error> {
        Ok(sqlx::query_as!(
            TotpCodeDetails,
            r#"
            SELECT id, description, last_used FROM totp
            WHERE state = 'enabled'
              AND user_id = $1
            "#,
            **user_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn delete_totp(
        &self,
        user_id: &ApiUserId,
        totp_id: &TotpId,
    ) -> Result<TotpId, Error> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM totp
            WHERE id = $2
              AND user_id = $1
            RETURNING id
            "#,
            **user_id,
            **totp_id
        )
        .fetch_one(&self.pool)
        .await?
        .into())
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
                   u.global_role AS "global_role: Role",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
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
            update
                .current_password
                .ok_or(Error::BadRequest(
                    "Must provide current password".to_string(),
                ))?
                .verify_password(&hash)
                .inspect_err(|err| {
                    tracing::trace!(user_id = user_id.to_string(), "wrong password: {}", err)
                })
                .map_err(|_| Error::BadRequest("wrong password".to_string()))?;
        }

        let hash = update.new_password.generate_hash();
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

    pub async fn initiate_password_reset(
        &self,
        email: &EmailAddress,
    ) -> Result<PwResetData, Error> {
        let reset_secret = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let reset_secret_hash = password_auth::generate_hash(reset_secret.as_bytes());

        let record = sqlx::query!(
            r#"
            WITH ins AS (
                INSERT INTO password_reset (id, api_user_id, reset_secret, created_at)
                SELECT gen_random_uuid(), u.id, $2, now()
                FROM api_users u
                    LEFT JOIN password_reset pr on u.id = pr.api_user_id
                WHERE u.email = $1
                -- Allow at most one reset link per minute
                  AND (pr.created_at IS NULL OR pr.created_at < now() - '1 min'::interval)
                ON CONFLICT (api_user_id) DO UPDATE
                SET created_at = now(),
                    id = gen_random_uuid(),
                    reset_secret = $2
                RETURNING id, api_user_id
            )
            SELECT ins.id, u.name
            FROM ins
            JOIN api_users u ON ins.api_user_id = u.id;
            "#,
            email.as_str(),
            reset_secret_hash,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(PwResetData {
            pw_reset_id: record.id.into(),
            reset_secret,
            user_name: record.name,
        })
    }

    pub async fn is_password_reset_active(
        &self,
        pw_reset_id: PwResetId,
    ) -> Result<ResetLinkCheck, Error> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT count(t.id) as "count!"
            FROM password_reset r
                LEFT JOIN totp t on r.api_user_id = t.user_id
            WHERE r.id = $1
              AND r.created_at > now() - '15 minutes'::interval
              AND (t.state IS NULL OR t.state = 'enabled')
            GROUP BY r.id
            "#,
            *pw_reset_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match count {
            None => Ok(ResetLinkCheck::NotActive),
            Some(0) => Ok(ResetLinkCheck::ActiveWithout2Fa),
            Some(_) => Ok(ResetLinkCheck::ActiveWith2Fa),
        }
    }

    pub async fn finish_password_reset(
        &self,
        pw_reset_id: PwResetId,
        reset_secret: Password,
        new_password: Password,
        totp_code: Option<TotpCode>,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        let Some(record) = sqlx::query!(
            r#"
            SELECT pwr.reset_secret, pwr.api_user_id, count(t.id) AS "totp_count!"
            FROM password_reset pwr
                LEFT JOIN totp t ON t.user_id = api_user_id
            WHERE pwr.id = $1
              AND pwr.created_at > now() - '15 minutes'::interval
              AND (t.state IS NULL OR t.state = 'enabled')
            GROUP BY pwr.id
            "#,
            *pw_reset_id
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(Error::NotFound("invalid password reset secret"));
        };

        reset_secret
            .verify_password(&record.reset_secret)
            .map_err(|_| Error::NotFound("invalid password reset secret"))?;

        if record.totp_count > 0 {
            let Some(totp_code) = totp_code else {
                return Err(Error::BadRequest("Missing TOTP code".to_string()));
            };
            if !self
                .check_totp_code(&record.api_user_id.into(), totp_code.as_ref())
                .await?
            {
                return Err(Error::BadRequest("Invalid TOTP code".to_string()));
            }
        }

        sqlx::query!(
            r#"
            UPDATE api_users
            SET password_hash = $1
            WHERE id = $2
            "#,
            new_password.generate_hash(),
            record.api_user_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM password_reset WHERE id = $1
            "#,
            *pw_reset_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn remove_password_reset_expired_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<(), Error> {
        trace!("Removing password reset links before {}", before);
        let rows = sqlx::query!(
            r#"
            DELETE FROM password_reset
            WHERE created_at < $1
            "#,
            before
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows > 0 {
            tracing::debug!("Removed {} password reset links", rows);
        }

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
            current_password
                .verify_password(&hash)
                .inspect_err(|err| trace!(user_id = user_id.to_string(), "wrong password: {}", err))
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
                   u.global_role AS "global_role: Role",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
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
                   u.global_role AS "global_role: Role",
                   u.password_hash IS NOT NULL AS "password_enabled!"
            FROM api_users u
                LEFT JOIN api_users_organizations o ON u.id = o.api_user_id
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
        struct HashAndCounter {
            hash: Option<String>,
            counter: i32,
        }

        let res = sqlx::query_as!(
            HashAndCounter,
            r#"
            UPDATE api_users
            SET password_try_counter       = CASE
                                             WHEN password_try_counter_reset < now() THEN 0
                                             ELSE password_try_counter + 1 END,
                password_try_counter_reset = CASE
                                             WHEN password_try_counter_reset < now() THEN now() + '1 min'
                                             ELSE password_try_counter_reset END
            WHERE email = $1
            RETURNING password_try_counter as counter, password_hash as hash
            "#,
            email.as_str()
        )
            .fetch_optional(&self.pool)
            .await?;

        if let Some(HashAndCounter {
            hash: Some(hash),
            counter,
        }) = res
        {
            if counter > 3 {
                // TODO, we might wan't to send an email to the user telling their account got temporarily blocked (see #222)
                // Note, we must not show any other behaviour to the outside world to avoid leaking if an account exists
                tracing::warn!(
                    attempts = counter,
                    "Too many failed password attempts for user {}",
                    email
                );
                return Err(Error::NotFound("User not found or wrong password"));
            }

            password
                .verify_password(&hash)
                .inspect_err(|err| tracing::trace!("wrong password for {}: {}", email, err))
                .map_err(|_| Error::NotFound("User not found or wrong password"))?;
            return Ok(());
        }
        Err(Error::NotFound("User not found or wrong password"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    impl ApiUser {
        pub fn new(global_role: Option<Role>, org_roles: Vec<OrgRole>) -> Self {
            Self {
                id: "0b8c948a-8f0c-4b63-a70e-78a9a186f7a2".parse().unwrap(),
                name: "Test Api User".to_string(),
                email: Some("test@test.com".parse().unwrap()),
                global_role,
                org_roles,
                github_user_id: None,
                password_enabled: false,
            }
        }
    }

    impl PartialEq<NewApiUser> for ApiUser {
        fn eq(&self, other: &NewApiUser) -> bool {
            let mut other_org_roles = other.org_roles.clone();
            other_org_roles.sort();

            let mut self_org_roles = self.org_roles.clone();
            self_org_roles.sort();

            self.github_user_id == other.github_user_id
                && self.email.as_ref() == Some(&other.email)
                && self.name == other.name
                && self.global_role == other.global_role
                && self_org_roles == other_org_roles
        }
    }

    impl Clone for NewApiUser {
        fn clone(&self) -> Self {
            Self {
                email: self.email.clone(),
                name: self.name.clone(),
                password: None,
                global_role: self.global_role,
                org_roles: self.org_roles.clone(),
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
            global_role: Some(Role::Admin),
            org_roles: vec![OrgRole {
                role: Role::Admin,
                org_id: "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap(),
            }],
            github_user_id: Some(123),
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);

        let user = NewApiUser {
            email: "test2@email.com".parse().unwrap(),
            name: "Test User 2".to_string(),
            password: None,
            global_role: None,
            org_roles: vec![],
            github_user_id: None,
        };

        let created = repo.create(user.clone()).await.unwrap();

        assert_eq!(created, user);
    }
}
