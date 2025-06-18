use crate::models::{ApiUserId, Error};
use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From, FromStr};
use serde::{Deserialize, Serialize};
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
    sqlx::Type,
    PartialOrd,
    Ord,
    Eq,
)]
#[sqlx(transparent)]
pub struct OrganizationId(Uuid);

impl OrganizationId {
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Organization {
    id: OrganizationId,
    pub name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Organization {
    pub fn id(&self) -> OrganizationId {
        self.id
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewOrganization {
    name: String,
}

#[derive(Debug, Clone)]
pub struct OrganizationRepository {
    pool: sqlx::PgPool,
}

#[derive(Debug, Deserialize, Default)]
pub struct OrganizationFilter {
    pub(crate) orgs: Option<Vec<OrganizationId>>,
}

impl OrganizationFilter {
    fn org_uuids(&self) -> Option<Vec<Uuid>> {
        self.orgs
            .as_deref()
            .map(|o| o.iter().map(|o| o.as_uuid()).collect())
    }
}

#[derive(Debug)]
pub enum QuotaStatus {
    Exceeded,
    Below(u64),
}

impl OrganizationRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn quota_status(&self, id: OrganizationId) -> Result<QuotaStatus, Error> {
        let messages_sent = sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!"
            FROM messages
            WHERE organization_id = $1
              AND created_at > date_trunc('month', current_date)
              AND status = 'delivered' OR status = 'processing'
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;

        // TODO make quota dependent on subscription
        let quota = 2_000_000;
        let remaining = quota - messages_sent;

        if remaining < 0 {
            Ok(QuotaStatus::Exceeded)
        } else {
            Ok(QuotaStatus::Below(remaining as u64))
        }
    }

    pub async fn create(&self, organization: NewOrganization) -> Result<Organization, Error> {
        Ok(sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (id, name)
            VALUES (gen_random_uuid(), $1)
            RETURNING *
            "#,
            organization.name,
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list(&self, filter: &OrganizationFilter) -> Result<Vec<Organization>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            Organization,
            r#"
            SELECT * FROM organizations
            WHERE ($1::uuid[] IS NULL OR id = ANY($1))
            ORDER BY name
            "#,
            orgs.as_deref(),
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_by_id(
        &self,
        id: OrganizationId,
        filter: &OrganizationFilter,
    ) -> Result<Option<Organization>, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_as!(
            Organization,
            r#"
            SELECT * FROM organizations
            WHERE id = $1
              AND ($2::uuid[] IS NULL OR id = ANY($2))
            "#,
            *id,
            orgs.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn remove(
        &self,
        id: OrganizationId,
        filter: &OrganizationFilter,
    ) -> Result<OrganizationId, Error> {
        let orgs = filter.org_uuids();
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM organizations
            WHERE id = $1
              AND ($2::uuid[] IS NULL OR id = ANY($2))
            RETURNING id
            "#,
            *id,
            orgs.as_deref(),
        )
        .fetch_one(&self.pool)
        .await?
        .into())
    }

    pub async fn add_user(&self, org_id: OrganizationId, user_id: ApiUserId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO api_users_organizations (organization_id, api_user_id, role)
            VALUES ($1, $2, 'admin')
            "#,
            *org_id,
            *user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn organization_lifecycle(db: PgPool) {
        let repo = OrganizationRepository::new(db);

        let org1 = repo
            .create(NewOrganization {
                name: "TestOrg1".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(org1.name, "TestOrg1");
        let org2 = repo
            .create(NewOrganization {
                name: "TestOrg2".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(org2.name, "TestOrg2");

        let orgs = repo.list(&Default::default()).await.unwrap();
        assert_eq!(orgs, vec![org1.clone(), org2.clone()]);

        repo.remove(org1.id, &Default::default()).await.unwrap();
        let orgs = repo.list(&Default::default()).await.unwrap();
        assert_eq!(orgs, vec![org2.clone()]);

        let not_found = repo.get_by_id(org1.id, &Default::default()).await.unwrap();
        assert_eq!(None, not_found);
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "org_domains",
            "projects",
            "streams",
            "smtp_credentials"
        )
    ))]
    async fn quota(db: PgPool) {
        let repo = OrganizationRepository::new(db.clone());

        let status = repo
            .quota_status("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
            .await
            .unwrap();

        assert!(matches!(status, QuotaStatus::Below(_)));

        sqlx::query!(
            r#"
            INSERT INTO messages (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, reason, from_email, recipients, raw_data, message_data, retry_after) 
            SELECT
                gen_random_uuid(),
                '44729d9f-a7dc-4226-b412-36a7537f5176'::uuid,
                'ed28baa5-57f7-413f-8c77-7797ba6a8780'::uuid,
                '3ba14adf-4de1-4fb6-8c20-50cc2ded5462'::uuid,
                '85785f4c-9167-4393-bbf2-3c3e21067e4a'::uuid,
                '9442cbbf-9897-4af7-9766-4ac9c1bf49cf'::uuid,
                'delivered',
                null,
                'test@remail.com',
                '{"asdf@example.com"}',
                ''::bytea,
                '{}'::jsonb,
                null
            FROM generate_series(1, 1_000_000);
            "#
        ).execute(&db).await.unwrap();

        let start_time = Utc::now().time();
        let status = repo
            .quota_status("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
            .await
            .unwrap();

        let end_time = Utc::now().time();
        let diff = end_time - start_time;
        println!("milliseconds to count 1 million matching messages {}", diff.num_milliseconds());

        assert!(matches!(status, QuotaStatus::Below(_)));

        sqlx::query!(
            r#"
            INSERT INTO messages (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, reason, from_email, recipients, raw_data, message_data, retry_after) 
            SELECT
                gen_random_uuid(),
                '44729d9f-a7dc-4226-b412-36a7537f5176'::uuid,
                'ed28baa5-57f7-413f-8c77-7797ba6a8780'::uuid,
                '3ba14adf-4de1-4fb6-8c20-50cc2ded5462'::uuid,
                '85785f4c-9167-4393-bbf2-3c3e21067e4a'::uuid,
                '9442cbbf-9897-4af7-9766-4ac9c1bf49cf'::uuid,
                'delivered',
                null,
                'test@remail.com',
                '{"asdf@example.com"}',
                ''::bytea,
                '{}'::jsonb,
                null
            FROM generate_series(1, 1_000_000);
            "#
        ).execute(&db).await.unwrap();

        let start_time = Utc::now().time();
        let status = repo
            .quota_status("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
            .await
            .unwrap();

        let end_time = Utc::now().time();
        let diff = end_time - start_time;
        println!("milliseconds to count 2 million matching messages {}", diff.num_milliseconds());

        assert!(matches!(status, QuotaStatus::Below(0)));

        sqlx::query!(
            r#"
            INSERT INTO messages (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, reason, from_email, recipients, raw_data, message_data, retry_after) 
            VALUES (
                gen_random_uuid(),
                '44729d9f-a7dc-4226-b412-36a7537f5176'::uuid,
                'ed28baa5-57f7-413f-8c77-7797ba6a8780'::uuid,
                '3ba14adf-4de1-4fb6-8c20-50cc2ded5462'::uuid,
                '85785f4c-9167-4393-bbf2-3c3e21067e4a'::uuid,
                '9442cbbf-9897-4af7-9766-4ac9c1bf49cf'::uuid,
                'delivered',
                null,
                'test@remail.com',
                '{"asdf@example.com"}',
                ''::bytea,
                '{}'::jsonb,
                null
                )
            "#
        ).execute(&db).await.unwrap();

        let status = repo
            .quota_status("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
            .await
            .unwrap();

        assert!(matches!(status, QuotaStatus::Exceeded));

        sqlx::query!(
            r#"
            INSERT INTO messages (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, reason, from_email, recipients, raw_data, message_data, retry_after) 
            SELECT
                gen_random_uuid(),
                '5d55aec5-136a-407c-952f-5348d4398204'::uuid,
                'ed28baa5-57f7-413f-8c77-7797ba6a8780'::uuid,
                '3ba14adf-4de1-4fb6-8c20-50cc2ded5462'::uuid,
                '85785f4c-9167-4393-bbf2-3c3e21067e4a'::uuid,
                '9442cbbf-9897-4af7-9766-4ac9c1bf49cf'::uuid,
                'delivered',
                null,
                'test@remail.com',
                '{"asdf@example.com"}',
                ''::bytea,
                '{}'::jsonb,
                null
            FROM generate_series(1, 3_000_000);
            "#
        ).execute(&db).await.unwrap();
        sqlx::query!(
            r#"
            INSERT INTO messages (id, organization_id, domain_id, project_id, stream_id, smtp_credential_id, status, reason, from_email, recipients, raw_data, message_data, retry_after) 
            SELECT
                gen_random_uuid(),
                '44729d9f-a7dc-4226-b412-36a7537f5176'::uuid,
                'ed28baa5-57f7-413f-8c77-7797ba6a8780'::uuid,
                '3ba14adf-4de1-4fb6-8c20-50cc2ded5462'::uuid,
                '85785f4c-9167-4393-bbf2-3c3e21067e4a'::uuid,
                '9442cbbf-9897-4af7-9766-4ac9c1bf49cf'::uuid,
                'held',
                null,
                'test@remail.com',
                '{"asdf@example.com"}',
                ''::bytea,
                '{}'::jsonb,
                null
            FROM generate_series(1, 1_000_000);
            "#
        ).execute(&db).await.unwrap();

        let start_time = Utc::now().time();
        let status = repo
            .quota_status("44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap())
            .await
            .unwrap();

        let end_time = Utc::now().time();
        let diff = end_time - start_time;
        println!("milliseconds to count 6 million messages (2 million matching) {}", diff.num_milliseconds());
    }
}
