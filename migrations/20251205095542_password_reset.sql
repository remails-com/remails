CREATE TABLE runtime_config
(
    -- Project that should be used to send emails to users of remails, such as password reset links, email verification, quota warings, etc.
    system_email_project uuid REFERENCES projects (id),
    system_email_address text
);

-- make sure the table row exists, even if no meaningfully content is provided
INSERT INTO runtime_config (system_email_project, system_email_address)
VALUES (NULL, NULL);

CREATE TABLE password_reset
(
    id           uuid PRIMARY KEY,
    api_user_id  uuid                     NOT NULL REFERENCES api_users (id) ON DELETE CASCADE UNIQUE,
    reset_secret text                     NOT NULL,
    created_at   timestamp with time zone NOT NULL
);