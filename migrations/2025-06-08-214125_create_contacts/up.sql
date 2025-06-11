CREATE TABLE contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    user_id_1 UUID NOT NULL,
    user_id_2 UUID NOT NULL,
    status TEXT NOT NULL,
    initiator UUID,
    FOREIGN KEY (user_id_1) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id_2) REFERENCES users (id) ON DELETE CASCADE,
    UNIQUE (user_id_1, user_id_2)
);
