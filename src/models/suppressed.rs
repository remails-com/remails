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
        email: &EmailAddress,
        org: OrganizationId,
    ) -> Result<(), Error> {
        const MAX_ATTEMPTS: i32 = 10;
        let retry_after = Utc::now() + Duration::days(30);

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
    /// Used after a successful delivery or a manual suppression removal via the API
    pub async fn unsuppress(&self, email: &EmailAddress, org: OrganizationId) -> Result<(), Error> {
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

    /// Check if we should suppress delivery to an email address in an organization
    ///
    /// An address is suppressed if `attempts_left` is 0. However, if `retry_after` is in the past, we will do another delivery attempt.
    pub async fn should_suppress(
        &self,
        email: &EmailAddress,
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
            return Ok(false); // not suppressed
        };

        Ok(record.attempts_left.unwrap_or(0) <= 0
            && record.retry_after.map_or(true, |t| t > Utc::now()))
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

    #[cfg(test)]
    async fn insert_suppression(
        &self,
        email: &EmailAddress,
        org: OrganizationId,
        retry_after: DateTime<Utc>,
        attempts: i32,
    ) -> Result<(), Error> {
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
            attempts,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_suppression_system(pool: PgPool) {
        const MAX_ATTEMPTS: i32 = 10;
        let repo = SuppressedRepository::new(pool);
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let org_2 = "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap();

        assert!(repo.list_suppressed(org_1).await.unwrap().is_empty());
        assert!(repo.list_suppressed(org_2).await.unwrap().is_empty());

        // email is not yet suppressed
        let bad_email: EmailAddress = "bad@example.com".parse().unwrap();
        assert!(!repo.should_suppress(&bad_email, org_1).await.unwrap());

        // report failures
        for _ in 0..MAX_ATTEMPTS - 1 {
            repo.report_failure(&bad_email, org_1).await.unwrap();
        }

        let suppressed = repo.list_suppressed(org_1).await.unwrap();
        assert_eq!(suppressed.len(), 1);
        assert_eq!(suppressed[0].attempts_left, 1);
        assert_eq!(suppressed[0].email_address, bad_email);
        assert_eq!(suppressed[0].organization_id, org_1);
        assert!(suppressed[0].retry_after.unwrap() > Utc::now() + Duration::days(29));
        assert!(repo.list_suppressed(org_2).await.unwrap().is_empty());

        // email is still not suppressed
        assert!(!repo.should_suppress(&bad_email, org_1).await.unwrap());

        // report one more failures
        repo.report_failure(&bad_email, org_1).await.unwrap();

        // email is suppressed
        let suppressed = repo.list_suppressed(org_1).await.unwrap();
        assert_eq!(suppressed.len(), 1);
        assert_eq!(suppressed[0].attempts_left, 0);
        assert!(repo.should_suppress(&bad_email, org_1).await.unwrap());

        // unsuppress email
        repo.unsuppress(&bad_email, org_1).await.unwrap();
        assert!(repo.list_suppressed(org_1).await.unwrap().is_empty());
        assert!(!repo.should_suppress(&bad_email, org_1).await.unwrap());

        // unsuppressing multiple times is allowed but does nothing
        repo.unsuppress(&bad_email, org_1).await.unwrap();
        assert!(repo.list_suppressed(org_1).await.unwrap().is_empty());
        assert!(!repo.should_suppress(&bad_email, org_1).await.unwrap());
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_retry_after(pool: PgPool) {
        let repo = SuppressedRepository::new(pool);
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();

        // should not yet retry
        let email: EmailAddress = "test1@example.com".parse().unwrap();
        repo.insert_suppression(&email, org_1, Utc::now() + Duration::days(1), 0)
            .await
            .unwrap();
        assert!(repo.should_suppress(&email, org_1).await.unwrap());

        // should retry now
        let email: EmailAddress = "test2@example.com".parse().unwrap();
        repo.insert_suppression(&email, org_1, Utc::now(), 0)
            .await
            .unwrap();
        assert!(!repo.should_suppress(&email, org_1).await.unwrap());

        // should clean up (should also retry now)
        let email: EmailAddress = "test3@example.com".parse().unwrap();
        repo.insert_suppression(&email, org_1, Utc::now() - Duration::days(30), 0)
            .await
            .unwrap();
        assert!(!repo.should_suppress(&email, org_1).await.unwrap());
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "api_users")))]
    async fn test_clean_up(pool: PgPool) {
        let repo = SuppressedRepository::new(pool);
        let org_1 = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();

        let keep: EmailAddress = "test1@example.com".parse().unwrap();
        repo.insert_suppression(&keep, org_1, Utc::now() - Duration::days(29), 0)
            .await
            .unwrap();

        let remove: EmailAddress = "test2@example.com".parse().unwrap();
        repo.insert_suppression(&remove, org_1, Utc::now() - Duration::days(30), 0)
            .await
            .unwrap();

        repo.clean_up_before(Utc::now() - Duration::days(30))
            .await
            .unwrap();

        let suppressed = repo.list_suppressed(org_1).await.unwrap();
        assert_eq!(suppressed.len(), 1);
        assert_eq!(suppressed[0].email_address, keep);
    }
}
