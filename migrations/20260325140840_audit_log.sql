CREATE TYPE audit_log_target_type AS ENUM (
    'project',
    'domain',
    'message',
    'smtp_credential',
    'api_key',
    'invite_link',
    'member'
);

CREATE TYPE audit_log_actor_type AS ENUM (
    'api_user',
    'api_key',
    'system'
);

CREATE TABLE audit_log
(
    id              uuid                  PRIMARY KEY,
    organization_id uuid                  NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    target_id       uuid,
    target_type     audit_log_target_type,
    actor_id        uuid,
    actor_type      audit_log_actor_type  NOT NULL,
    action          text                  NOT NULL,
    details         jsonb,
    occurred_at     timestamptz           NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_organization_occurred_at_idx
    ON audit_log (organization_id, occurred_at DESC);

CREATE INDEX audit_logs_target_idx
    ON audit_log (organization_id, target_type, target_id, occurred_at DESC);

CREATE INDEX audit_logs_actor_idx
    ON audit_log (organization_id, actor_type, actor_id, occurred_at DESC);
