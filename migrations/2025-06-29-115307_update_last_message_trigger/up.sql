CREATE OR REPLACE FUNCTION update_last_message()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE talks
        SET last_message_id = NEW.id
        WHERE id = NEW.talk_id
          AND (
              last_message_id IS NULL OR
              NEW.created_at > COALESCE((SELECT created_at FROM messages WHERE id = talks.last_message_id), 'epoch')
          );
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE talks
        SET last_message_id = (
            SELECT id FROM messages
            WHERE talk_id = OLD.talk_id AND id != OLD.id
            ORDER BY created_at DESC
            LIMIT 1
        )
        WHERE id = OLD.talk_id AND last_message_id = OLD.id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_last_message
AFTER INSERT OR DELETE ON messages
FOR EACH ROW
EXECUTE FUNCTION update_last_message();
