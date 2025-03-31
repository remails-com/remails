use sqlx::PgPool;
use tracing::info;

const FIXTURES: &[(&str, &str)] = &[
    ("organizations.sql", include_str!("fixtures/organizations.sql")),
    ("domains.sql", include_str!("fixtures/domains.sql")),
    ("api_users.sql", include_str!("fixtures/api_users.sql")),
    ("smtp_credential.sql", include_str!("fixtures/smtp_credential.sql")),
    ("messages.sql", include_str!("fixtures/messages.sql")),
];

pub async fn load_fixtures(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for (name, sql) in FIXTURES {
        info!("Loading fixture: {name}");
        sqlx::raw_sql(sql).execute(pool).await?;
    }

    info!("All fixtures loaded successfully");

    Ok(())
}