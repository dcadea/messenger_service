CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    sub TEXT NOT NULL UNIQUE,
    nickname TEXT NOT NULL,
    name TEXT NOT NULL,
    picture TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE
);
