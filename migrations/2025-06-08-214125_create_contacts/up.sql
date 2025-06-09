CREATE TABLE contacts (
    user_id_1 UUID,
    user_id_2 UUID,
    status TEXT NOT NULL,
    FOREIGN KEY (user_id_1) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id_2) REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (user_id_1, user_id_2)
);
