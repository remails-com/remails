CREATE TABLE runtime_config
(
    -- Project that should be used to send emails to users of remails, such as password reset links, email verification, quota warings, etc.
    system_email_project uuid references projects (id),
    system_email_address text
);

-- make sure the table row exists, even if no meaningfully content is provided
INSERT INTO runtime_config (system_email_project, system_email_address)
VALUES (NULL, NULL);

ALTER TABLE api_users
    ADD password_reset_secret text,
    ADD password_reset_time   timestamp with time zone