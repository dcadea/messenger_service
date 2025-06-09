CREATE TABLE chats (id UUID PRIMARY KEY REFERENCES talks (id));

CREATE TABLE chats_users (
    chat_id UUID,
    user_id UUID,
    FOREIGN KEY (chat_id) REFERENCES chats (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (chat_id, user_id)
);
