CREATE TYPE talk_kind AS ENUM ('group', 'chat');

CREATE TABLE talks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    kind talk_kind NOT NULL,
    last_message_id UUID
);
