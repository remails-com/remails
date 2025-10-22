ALTER TABLE messages ADD api_key_id uuid REFERENCES api_keys (id) ON DELETE SET NULL;

ALTER TABLE messages ALTER COLUMN status SET DEFAULT 'processing';
