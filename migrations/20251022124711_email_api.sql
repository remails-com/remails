ALTER TABLE messages ADD api_key_id uuid REFERENCES api_keys (id) ON DELETE SET NULL;

-- messages should start out as processing
ALTER TABLE messages ALTER COLUMN status SET DEFAULT 'processing';

-- store email Message ID header in database
ALTER TABLE messages ADD message_id_header varchar;
