ALTER TABLE smtp_credentials
    DROP CONSTRAINT smtp_credentials_stream_id_fkey;

ALTER TABLE smtp_credentials
    ADD FOREIGN KEY (stream_id) REFERENCES streams
        ON DELETE CASCADE;

ALTER TABLE domains
    DROP CONSTRAINT domains_project_id_fkey;

ALTER TABLE domains
    ADD FOREIGN KEY (project_id) REFERENCES projects
        ON DELETE CASCADE;

ALTER TABLE organizations
    ADD CONSTRAINT check_name
        CHECK (char_length(name) >= 3 AND char_length(name) <= 50);

ALTER TABLE projects
    ADD CONSTRAINT check_name
        CHECK (char_length(name) >= 3 AND char_length(name) <= 50);

ALTER TABLE streams
    ADD CONSTRAINT check_name
        CHECK (char_length(name) >= 3 AND char_length(name) <= 50);