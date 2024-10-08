CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION update_updated_at_column()   
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;   
END;
$$ language 'plpgsql';

DROP TABLE IF EXISTS users CASCADE;
CREATE TABLE users (
    id uuid PRIMARY KEY NOT NULL,
    username varchar NOT NULL UNIQUE CHECK (username ~ '^[a-zA-Z0-9_-]{2,128}$'),
    password_hash varchar NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_users_updated_at
BEFORE UPDATE ON users FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();

CREATE TYPE message_status AS ENUM (
    'processing',
    'held',
    'accepted',
    'rejected',
    'delivered',
    'failed'
);

DROP TABLE IF EXISTS messages CASCADE;
CREATE TABLE messages (
    id uuid PRIMARY KEY NOT NULL,
    user_id uuid NOT NULL REFERENCES users(id),
    status message_status NOT NULL,
    from_email varchar NOT NULL,
    recipients varchar[] NOT NULL,
    raw_data bytea,
    message_data jsonb,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TRIGGER update_messages_updated_at
BEFORE UPDATE ON messages FOR EACH ROW EXECUTE PROCEDURE update_updated_at_column();
