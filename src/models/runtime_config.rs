use crate::models::{OrganizationId, ProjectId, error::Error};
use garde::Validate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(ToSchema, Validate, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct RuntimeConfig {
    #[garde(skip)]
    system_email_project: Option<ProjectId>,
    #[garde(email)]
    system_email_address: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
#[cfg_attr(test, derive(Deserialize, PartialEq, Eq))]
pub struct RuntimeConfigResponse {
    system_email_project: Option<ProjectId>,
    system_email_project_name: Option<String>,
    system_email_organization: Option<OrganizationId>,
    system_email_address: Option<String>,
}

pub struct RuntimeConfigRepository {
    pool: PgPool,
}

impl RuntimeConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self) -> Result<RuntimeConfigResponse, Error> {
        Ok(sqlx::query_as!(
            RuntimeConfigResponse,
            r#"
            SELECT
                system_email_address,
                system_email_project AS "system_email_project:ProjectId",
                p.name AS system_email_project_name,
                p.organization_id AS "system_email_organization:OrganizationId"
            FROM runtime_config 
                LEFT JOIN projects p ON p.id = system_email_project
            "#
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn update(&self, config: RuntimeConfig) -> Result<RuntimeConfigResponse, Error> {
        Ok(sqlx::query_as!(
            RuntimeConfigResponse,
            r#"
            UPDATE runtime_config rc
            SET system_email_address = $1,
                system_email_project = $2
            FROM runtime_config
                     LEFT JOIN projects p ON p.id = $2
            RETURNING
                rc.system_email_address,
                rc.system_email_project AS "system_email_project:ProjectId",
                p.name AS "system_email_project_name?",
                p.organization_id AS "system_email_organization?:OrganizationId";
            "#,
            config.system_email_address,
            config.system_email_project.map(|c| c.as_uuid())
        )
        .fetch_one(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{OrganizationId, ProjectId, RuntimeConfig, RuntimeConfigResponse};

    impl RuntimeConfigResponse {
        pub fn new(
            system_email_project: Option<ProjectId>,
            system_email_project_name: Option<String>,
            system_email_organization: Option<OrganizationId>,
            system_email_address: Option<String>,
        ) -> Self {
            Self {
                system_email_project,
                system_email_project_name,
                system_email_organization,
                system_email_address,
            }
        }
    }

    impl RuntimeConfig {
        pub fn new(
            system_email_project: Option<ProjectId>,
            system_email_address: Option<String>,
        ) -> Self {
            Self {
                system_email_project,
                system_email_address,
            }
        }
    }
}
