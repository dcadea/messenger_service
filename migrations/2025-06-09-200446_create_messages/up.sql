CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    talk_id UUID NOT NULL,
    owner UUID NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    seen BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (talk_id) REFERENCES talks (id) ON DELETE CASCADE,
    FOREIGN KEY (owner) REFERENCES users (id)
);
