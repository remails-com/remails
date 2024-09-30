CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION update_updated_at_column()   
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;   
END;
$$ language 'plpgsql';

CREATE TABLE users (
    id uuid PRIMARY KEY NOT NULL,
    username varchar NOT NULL CHECK (username ~ '^[a-zA-Z0-9_-]{2,128}$'),
    password_hash varchar NOT NULL,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now()
);

CREATE TRIGGER update_users_updated_at
BEFORE UPDATE ON users FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TABLE messages (
    id uuid PRIMARY KEY NOT NULL,
    from_email varchar NOT NULL,
    recipients varchar NOT NULL,
    raw_data bytea NOT NULL,
    message_data jsonb,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now()
);

CREATE TRIGGER update_messages_updated_at
BEFORE UPDATE ON messages FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();
