CREATE TABLE statistics (
    organization_id  uuid        NOT NULL,
    project_id       uuid        NOT NULL,
    month            DATE        NOT NULL,
    statistics       jsonb       NOT NULL,
    PRIMARY KEY (organization_id, project_id, month)
);

CREATE INDEX index_message_statistics ON messages (organization_id, project_id, created_at);
CREATE INDEX index_statistics ON statistics (organization_id, project_id, month);

ALTER TABLE projects
ADD COLUMN retention_period_days INTEGER NOT NULL DEFAULT 30;
