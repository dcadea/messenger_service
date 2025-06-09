CREATE TABLE groups (
    id UUID PRIMARY KEY REFERENCES talks (id),
    owner UUID NOT NULL,
    name TEXT NOT NULL,
    FOREIGN KEY (owner) REFERENCES users (id)
);

CREATE TABLE groups_users (
    group_id UUID,
    user_id UUID,
    FOREIGN KEY (group_id) REFERENCES groups (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, user_id)
);
