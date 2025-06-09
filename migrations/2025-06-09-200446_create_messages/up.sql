CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    talk_id UUID NOT NULL,
    owner UUID NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    seen BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (talk_id) REFERENCES talks (id),
    FOREIGN KEY (owner) REFERENCES users (id)
);

ALTER TABLE talks ADD CONSTRAINT fk_last_message FOREIGN KEY (last_message_id) REFERENCES messages (id) DEFERRABLE INITIALLY DEFERRED;
