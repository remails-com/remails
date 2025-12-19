use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use tracing::debug;
use utoipa::ToSchema;

use crate::models::{Error, OrganizationId, ProjectId};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(test, derive(PartialEq, serde::Deserialize))]
pub struct StatisticsEntry {
    organization_id: OrganizationId,
    project_id: ProjectId,
    date: NaiveDate,
    pub statistics: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(test, derive(PartialEq, serde::Deserialize))]
pub struct Statistics {
    pub monthly: Vec<StatisticsEntry>,
    pub daily: Vec<StatisticsEntry>,
}

#[cfg(test)]
impl Statistics {
    pub fn sort(&mut self) {
        self.monthly
            .sort_by_key(|stat| (*stat.project_id, stat.date));
        self.daily.sort_by_key(|stat| (*stat.project_id, stat.date));
    }
}

#[derive(Debug, Clone)]
pub struct StatisticsRepository {
    pool: PgPool,
}

impl StatisticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_stats(&self, organization_id: OrganizationId) -> Result<Statistics, Error> {
        Ok(Statistics {
            monthly: self.get_monthly_stats(organization_id).await?,
            daily: self.get_daily_stats(organization_id).await?,
        })
    }

    /// Gets daily stats for the past 30 days
    async fn get_daily_stats(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Vec<StatisticsEntry>, Error> {
        Ok(sqlx::query_as!(
            StatisticsEntry,
            r#"
            SELECT organization_id, project_id, day AS "date!",
                    jsonb_object_agg(status, count_per_status) AS statistics
            FROM (
                SELECT
                    organization_id, project_id, status,
                    date_trunc('day', created_at)::date AS day,
                    COUNT(*) AS count_per_status
                FROM messages
                GROUP BY
                    organization_id, project_id, status,
                    date_trunc('day', created_at)::date
            )
            WHERE organization_id = $1 AND day > NOW() - INTERVAL '30 days'
            GROUP BY organization_id, project_id, day;
            "#,
            *organization_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Gets monthly stats from both the archived statistics as well as the live message data
    async fn get_monthly_stats(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Vec<StatisticsEntry>, Error> {
        Ok(sqlx::query_as!(
            StatisticsEntry,
            r#"
            SELECT organization_id AS "organization_id!",
                    project_id AS "project_id!",
                    month AS "date!",
                    statistics AS "statistics!"
            FROM (
                -- archived statistics
                SELECT * FROM statistics

                UNION ALL

                -- live statistics
                SELECT organization_id, project_id, month AS "month!",
                    jsonb_object_agg(status, count_per_status) AS statistics
                FROM (
                    SELECT
                        organization_id, project_id, status,
                        date_trunc('month', created_at)::date AS month,
                        COUNT(*) AS count_per_status
                    FROM messages
                    GROUP BY
                        organization_id, project_id, status,
                        date_trunc('month', created_at)::date
                )
                GROUP BY organization_id, project_id, month
            ) stats
            WHERE stats.organization_id = $1;
            "#,
            *organization_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn aggregate_and_archive_messages(&self) -> Result<(), Error> {
        let last_active = Utc::now() - Duration::days(30);
        let start_of_month = NaiveDate::from_ymd_opt(last_active.year(), last_active.month(), 1)
            .ok_or_else(|| Error::Internal(format!("Invalid date: {:?}", last_active)))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| Error::Internal(format!("Invalid date: {:?}", last_active)))?;
        let cutoff = DateTime::<Utc>::from_naive_utc_and_offset(start_of_month, Utc);

        let mut tx = self.pool.begin().await?;

        let gathered_rows = sqlx::query!(
            r#"
            INSERT INTO statistics (organization_id, project_id, month, statistics)
            SELECT organization_id, project_id, month,
                jsonb_object_agg(status, count_per_status) AS statistics
            FROM (
                SELECT
                    organization_id, project_id, status,
                    date_trunc('month', created_at)::date AS month,
                    COUNT(*) AS count_per_status
                FROM messages
                WHERE created_at < $1
                GROUP BY
                    organization_id, project_id, status,
                    date_trunc('month', created_at)::date
            )
            GROUP BY organization_id, project_id, month
            ON CONFLICT (organization_id, project_id, month)
            DO UPDATE SET statistics = EXCLUDED.statistics;
            "#,
            cutoff
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        debug!("gathered statistics for {} projects/months", gathered_rows);

        let deleted_rows = sqlx::query!(
            r#"
            DELETE FROM messages WHERE created_at < $1;
            "#,
            cutoff
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        debug!("Deleted {} messages", deleted_rows);

        tx.commit().await?;

        Ok(())
    }

    #[cfg(test)]
    async fn count_messages(&self) -> Result<i64, Error> {
        Ok(
            sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM messages"#)
                .fetch_one(&self.pool)
                .await?,
        )
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{bus::client::BusClient, periodically::Periodically, test::TestProjects};

    use super::*;

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "smtp_credentials",
            "messages"
        )
    ))]
    async fn test_get_statistics_before_and_after_removal(pool: PgPool) {
        let repo = StatisticsRepository::new(pool.clone());
        let bus_client = BusClient::new_from_env_var().unwrap();
        let periodically = Periodically::new(pool, bus_client).await.unwrap();

        let (org_1, proj_1) = TestProjects::Org1Project1.get_ids();

        // get statistics from messages
        let mut stats = repo.get_monthly_stats(org_1).await.unwrap();
        assert_eq!(stats.len(), 4);
        assert!(stats.iter().all(|stat| stat.organization_id == org_1));
        stats.sort_by_key(|stat| (*stat.project_id, stat.date));

        assert_eq!(stats[0].project_id, proj_1);
        assert_eq!(
            stats[0].statistics,
            json!({
                "held": 1,
                "processing": 1,
                "reattempt": 3,
            })
        );

        // count number of messages
        let nr_of_messages = repo.count_messages().await.unwrap();

        // clean up messages
        periodically.clean_up().await.unwrap();

        // count number of messages again
        let new_nr_of_messages = repo.count_messages().await.unwrap();
        assert_eq!(new_nr_of_messages, nr_of_messages - 1); // 1 message has been deleted

        // get statistics from messages again (should still be the same)
        let mut new_stats = repo.get_monthly_stats(org_1).await.unwrap();
        assert!(new_stats.iter().all(|stat| stat.organization_id == org_1));
        new_stats.sort_by_key(|stat| (*stat.project_id, stat.date));

        assert_eq!(stats, new_stats);
    }
}
