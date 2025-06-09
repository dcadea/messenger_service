CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    sub text NOT NULL UNIQUE,
    nickname text NOT NULL,
    name text NOT NULL,
    picture text NOT NULL,
    email text NOT NULL UNIQUE
)
