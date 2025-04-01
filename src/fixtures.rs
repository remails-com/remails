use sqlx::PgPool;
use tracing::info;

const FIXTURES: &[(&str, &str)] = &[
    (
        "organizations.sql",
        include_str!("fixtures/organizations.sql"),
    ),
    ("domains.sql", include_str!("fixtures/domains.sql")),
    ("api_users.sql", include_str!("fixtures/api_users.sql")),
    (
        "smtp_credential.sql",
        include_str!("fixtures/smtp_credential.sql"),
    ),
    ("messages.sql", include_str!("fixtures/messages.sql")),
];

pub async fn load_fixtures(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Running migrations...");
    sqlx::migrate!().run(pool).await?;
    info!("...done running migrations");

    info!("Loading fixtures...");
    for (name, sql) in FIXTURES {
        info!("Loading fixture: {name}");
        sqlx::raw_sql(sql).execute(pool).await?;
    }

    info!("..all fixtures loaded successfully");

    Ok(())
}
