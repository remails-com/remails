CREATE TYPE totp_state AS ENUM (
    'enrolling',
    'enabled'
    );

CREATE TABLE totp
(
    id          uuid PRIMARY KEY,
    description text        NOT NULL CHECK ( char_length(description) < 50 ),
    user_id     uuid        NOT NULL REFERENCES api_users (id),
    url         varchar     NOT NULL,
    state       totp_state  NOT NULL DEFAULT 'enrolling',
    last_used   timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER update_totp_updated_at
    BEFORE UPDATE
    ON totp
    FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();

ALTER TABLE api_users
    ADD password_try_counter       int         NOT NULL DEFAULT 0,
    ADD password_try_counter_reset timestamptz NOT NULL DEFAULT now(),
    ADD totp_try_counter           int         NOT NULL DEFAULT 0,
    ADD totp_try_counter_reset     timestamptz NOT NULL DEFAULT now();