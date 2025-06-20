CREATE TABLE chats (id UUID PRIMARY KEY REFERENCES talks (id));

CREATE TABLE chats_users (
    chat_id UUID,
    user_id UUID,
    FOREIGN KEY (chat_id) REFERENCES chats (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (chat_id, user_id)
);

CREATE INDEX idx_chats_users_chat_id ON chats_users (chat_id);
CREATE INDEX idx_chats_users_user_id ON chats_users (user_id);
