CREATE TABLE suppressed_email_addresses (
    email_address VARCHAR NOT NULL,
    organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    retry_after  timestamptz,
    attempts_left int,
    PRIMARY KEY (email_address, organization_id)
);
