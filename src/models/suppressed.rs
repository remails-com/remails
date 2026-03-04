use chrono::{DateTime, Duration, Utc};
use email_address::EmailAddress;

use crate::models::{Error, OrganizationId};

pub struct SuppressedEmailAddress {
    email_address: EmailAddress,
    organization_id: OrganizationId,
    retry_after: Option<DateTime<Utc>>,
    attempts_left: i32,
}

#[derive(Debug, Clone)]
pub struct SuppressedRepository {
    pool: sqlx::PgPool,
}

impl SuppressedRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Report a delivery failure for an email address within an organization
    ///
    /// When there are enough consecutive delivery failures, it will run out of attempts and suppress further emails.
    /// Suppressed emails are still tried once per day to check if they now work.
    pub async fn report_failure(
        &self,
        email: EmailAddress,
        org: OrganizationId,
    ) -> Result<(), Error> {
        const MAX_ATTEMPTS: i32 = 10;
        let retry_after = Utc::now() + Duration::days(1);

        sqlx::query!(
            r#"
            INSERT INTO suppressed_email_addresses (email_address, organization_id, retry_after, attempts_left)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (email_address, organization_id)
            DO UPDATE SET
                retry_after = EXCLUDED.retry_after,
                attempts_left = GREATEST(suppressed_email_addresses.attempts_left - 1, 0)
            "#,
            email.as_str(),
            *org,
            retry_after,
            MAX_ATTEMPTS - 1
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Unsuppress an email address within an organization
    ///
    /// Used after a successful delivery or a manual removal via the API
    pub async fn unsuppress(&self, email: EmailAddress, org: OrganizationId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            DELETE FROM suppressed_email_addresses
            WHERE email_address = $1 AND organization_id = $2
            "#,
            email.as_str(),
            *org
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if we should attempt delivery to an email address in an organization
    ///
    /// An address is suppressed if `attempts_left` is 0. However, if `retry_after` is in the past, we will do another delivery attempt.
    pub async fn should_attempt_delivery(
        &self,
        email: EmailAddress,
        org: OrganizationId,
    ) -> Result<bool, Error> {
        let Some(record) = sqlx::query!(
            r#"
            SELECT attempts_left, retry_after
            FROM suppressed_email_addresses
            WHERE email_address = $1 AND organization_id = $2
            "#,
            email.as_str(),
            *org
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(true); // not suppressed
        };

        Ok(record.attempts_left.unwrap_or(0) > 0
            || record.retry_after.map_or(false, |t| t <= Utc::now()))
    }

    /// List all suppressed email addresses within an organization
    pub async fn list_suppressed(
        &self,
        org: OrganizationId,
    ) -> Result<Vec<SuppressedEmailAddress>, Error> {
        let rows = sqlx::query!(
            r#"
            SELECT email_address, organization_id, retry_after, attempts_left
            FROM suppressed_email_addresses
            WHERE organization_id = $1
            "#,
            *org
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                Ok(SuppressedEmailAddress {
                    email_address: r.email_address.parse()?,
                    organization_id: r.organization_id.into(),
                    retry_after: r.retry_after,
                    attempts_left: r.attempts_left.unwrap_or(0),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?)
    }

    /// Clean up all suppressed email addresses whose `retry_after` is before `before`,
    /// meaning that the suppressed email address was not used since `before`
    pub async fn clean_up_before(&self, before: DateTime<Utc>) -> Result<(), Error> {
        tracing::trace!("Removing suppressed email addresses last used before {before}");
        let rows = sqlx::query!(
            r#"
            DELETE FROM suppressed_email_addresses
            WHERE retry_after < $1
            "#,
            before
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows > 0 {
            tracing::debug!("Removed {rows} unused suppressed email addresses");
        }

        Ok(())
    }
}
