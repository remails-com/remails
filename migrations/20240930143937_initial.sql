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


CREATE TYPE dkim_key_type AS ENUM (
    'rsa_sha256',
    'ed25519'
    );

CREATE TABLE domains
(
    id              uuid PRIMARY KEY,
    domain          varchar                  NOT NULL,
    organization_id uuid REFERENCES organizations (id) ON DELETE CASCADE,
    dkim_key_type   dkim_key_type,
    dkim_pkcs8_der  bytea,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    updated_at      timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_domains_updated_at
    BEFORE UPDATE
    ON domains
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();


CREATE TABLE api_users
(
    id             uuid PRIMARY KEY,
    email          varchar                  NOT NULL UNIQUE,
    roles          jsonb                    NOT NULL,
    github_user_id bigint,
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
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    PRIMARY KEY (api_user_id, organization_id)
);


CREATE TABLE smtp_credential
(
    id            uuid PRIMARY KEY         NOT NULL,
    domain_id     uuid                     NOT NULL REFERENCES domains (id),
    username      varchar                  NOT NULL UNIQUE CHECK (username ~ '^[a-zA-Z0-9_-]{2,128}$'),
    password_hash varchar                  NOT NULL,
    created_at    timestamp with time zone NOT NULL DEFAULT now(),
    updated_at    timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_smtp_credential_updated_at
    BEFORE UPDATE
    ON smtp_credential
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
    smtp_credential_id uuid                     NOT NULL REFERENCES smtp_credential (id),
    organization_id    uuid                     NOT NULL REFERENCES organizations (id),
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
