-- the message_id_header should always be specified since #369
ALTER TABLE messages
ALTER COLUMN message_id_header
SET NOT NULL;
