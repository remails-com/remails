-- Add a table update notification function
CREATE OR REPLACE FUNCTION table_update_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('table_update', json_build_object('table', TG_TABLE_NAME, 'action_type', TG_OP, 'data', to_json(NEW))::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER notify_message_update ON messages;
CREATE TRIGGER notify_message_update
    AFTER UPDATE
    ON messages
    FOR EACH ROW
EXECUTE PROCEDURE table_update_notify();
