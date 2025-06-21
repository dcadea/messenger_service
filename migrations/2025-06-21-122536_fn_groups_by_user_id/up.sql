CREATE OR REPLACE FUNCTION groups_by_user_id(_user_id UUID)
RETURNS TABLE (
    id UUID,
    last_message_id UUID,
    owner UUID,
    name TEXT,
    members UUID[]
)
AS $$
    SELECT
        t.id,
        t.last_message_id,
        g.owner AS owner,
        g.name AS name,
        array_agg(gu.user_id) AS members
    FROM talks t
    JOIN groups g ON g.id = t.id
    JOIN groups_users gu_self ON gu_self.group_id = t.id AND gu_self.user_id = _user_id
    JOIN groups_users gu ON gu.group_id = t.id
    WHERE t.kind = 'group'
    GROUP BY t.id, g.owner, g.name;
$$ LANGUAGE sql STABLE;
