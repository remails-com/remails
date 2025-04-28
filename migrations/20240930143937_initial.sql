CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION update_updated_at_column()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TABLE organizations
(
    id         uuid PRIMARY KEY,
    name       varchar                  NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE
    ON organizations
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();


CREATE TYPE role AS ENUM (
    'admin'
    );

CREATE TYPE org_role AS
(
    org_id uuid,
    role   role
);

CREATE TABLE api_users
(
    id             uuid PRIMARY KEY,
    name           varchar                  NOT NULL,
    email          varchar                  NOT NULL UNIQUE,
    github_user_id bigint,
    password_hash  varchar,
    created_at     timestamp with time zone NOT NULL DEFAULT now(),
    updated_at     timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TRIGGER update_api_users_updated_at
    BEFORE UPDATE
    ON api_users
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TABLE api_users_organizations
(
    api_user_id     uuid                     NOT NULL REFERENCES api_users (id) ON DELETE CASCADE,
    organization_id uuid                     NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    role            role                     NOT NULL,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    updated_at      timestamp with time zone NOT NULL DEFAULT now(),
    PRIMARY KEY (api_user_id, organization_id)
);
CREATE TRIGGER update_api_users_organizations_updated_at
    BEFORE UPDATE
    ON api_users_organizations
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TABLE api_users_global_roles
(
    api_user_id uuid PRIMARY KEY REFERENCES api_users (id) ON DELETE CASCADE,
    role        role                     NOT NULL,
    created_at  timestamp with time zone NOT NULL DEFAULT now(),
    updated_at  timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TRIGGER update_api_users_global_role_updated_at
    BEFORE UPDATE
    ON api_users_global_roles
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TABLE projects
(
    id              uuid PRIMARY KEY,
    organization_id uuid                     NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    name            varchar                  NOT NULL,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    updated_at      timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT unique_project_name UNIQUE (organization_id, name)
);

CREATE TRIGGER update_projects_updated_at
    BEFORE UPDATE
    ON projects
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TYPE dkim_key_type AS ENUM (
    'rsa_sha256',
    'ed25519'
    );

CREATE TABLE domains
(
    id              uuid PRIMARY KEY,
    domain          varchar                  NOT NULL UNIQUE,
    organization_id uuid REFERENCES organizations (id) ON DELETE CASCADE,
    project_id      uuid REFERENCES projects (id),
    CONSTRAINT either_organization_or_project CHECK ( (organization_id IS NULL) != (project_id IS NULL) ),
    dkim_key_type   dkim_key_type            NOT NULL,
    dkim_pkcs8_der  bytea                    NOT NULL,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    updated_at      timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_domains_updated_at
    BEFORE UPDATE
    ON domains
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();


CREATE TABLE streams
(
    id         uuid PRIMARY KEY,
    project_id uuid                     NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name       varchar                  NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_streams_updated_at
    BEFORE UPDATE
    ON streams
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TABLE smtp_credentials
(
    id            uuid PRIMARY KEY         NOT NULL,
    stream_id     uuid                     NOT NULL REFERENCES streams (id),
    description   varchar                  NOT NULL,
    username      varchar                  NOT NULL UNIQUE CHECK (username ~ '^[a-zA-Z0-9_-]{2,128}$'),
    password_hash varchar                  NOT NULL,
    created_at    timestamp with time zone NOT NULL DEFAULT now(),
    updated_at    timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_smtp_credential_updated_at
    BEFORE UPDATE
    ON smtp_credentials
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

CREATE TYPE message_status AS ENUM (
    'processing',
    'held',
    'accepted',
    'rejected',
    'delivered',
    'failed'
    );

CREATE TABLE messages
(
    id                 uuid PRIMARY KEY         NOT NULL,
    organization_id    uuid                     NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    domain_id          uuid                     REFERENCES domains (id) ON DELETE SET NULL,
    project_id         uuid                     NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    stream_id          uuid                     NOT NULL REFERENCES streams (id) ON DELETE CASCADE,
    smtp_credential_id uuid                     REFERENCES smtp_credentials (id) ON DELETE SET NULL,
    delivery_status    jsonb                    NOT NULL default '[]',
    status             message_status           NOT NULL,
    from_email         varchar                  NOT NULL,
    recipients         varchar[]                NOT NULL,
    raw_data           bytea                    NOT NULL,
    message_data       jsonb,
    created_at         timestamp with time zone NOT NULL DEFAULT now(),
    updated_at         timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_messages_updated_at
    BEFORE UPDATE
    ON messages
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();
