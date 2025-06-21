CREATE OR REPLACE FUNCTION chats_for_user(_user_id UUID)
RETURNS TABLE (
    id UUID,
    last_message_id UUID,
    recipient UUID,
    name TEXT,
    picture TEXT
)
AS $$
    SELECT
        t.id,
        t.last_message_id,
        u.id AS recipient,
        u.name AS name,
        u.picture AS picture
    FROM talks t
    JOIN chats c ON c.id = t.id
    JOIN chats_users cu_self ON cu_self.chat_id = t.id AND cu_self.user_id = _user_id
    JOIN chats_users cu_other ON cu_other.chat_id = t.id AND cu_other.user_id != _user_id
    JOIN users u ON u.id = cu_other.user_id
    WHERE t.kind = 'chat';
$$ LANGUAGE sql STABLE;
