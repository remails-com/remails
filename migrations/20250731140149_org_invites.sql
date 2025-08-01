CREATE TABLE organization_invites(
    id              uuid                     PRIMARY KEY,
    organization_id uuid                     NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    password_hash   varchar                  NOT NULL,
    created_by      uuid                     NOT NULL REFERENCES api_users (id) ON DELETE CASCADE,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    expires_at      timestamp with time zone NOT NULL
);
