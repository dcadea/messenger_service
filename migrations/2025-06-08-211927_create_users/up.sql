CREATE TABLE users (
    id UUID PRIMARY KEY,
    sub text NOT NULL,
    nickname text NOT NULL,
    picture text NOT NULL,
    email text NOT NULL
)
