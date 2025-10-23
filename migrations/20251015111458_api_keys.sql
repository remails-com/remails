CREATE TABLE api_keys (
    id              uuid PRIMARY KEY,
    description     varchar                  NOT NULL,
    password_hash   varchar                  NOT NULL,
    organization_id uuid                     NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    role            role                     NOT NULL,
    created_at      timestamp with time zone NOT NULL DEFAULT now(),
    updated_at      timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_api_keys_updated_at
BEFORE UPDATE ON api_keys FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();
