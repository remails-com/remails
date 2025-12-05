CREATE TABLE global_config
(
    -- Project that should be used to send emails to users of remails, such as password reset links, email verification, quota warings, etc.
    internal_email_project uuid,
    internal_email_address text
);

-- make sure the table row exists, even if no meaningfully content is provided
INSERT INTO global_config (internal_email_project, internal_email_address) VALUES (NULL, NULL);