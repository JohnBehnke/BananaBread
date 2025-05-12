CREATE TABLE IF NOT EXISTS media (
    id           VARCHAR(36) PRIMARY KEY NOT NULL,
    name         TEXT    NOT NULL,
    path         TEXT    NOT NULL UNIQUE,
    alias        TEXT
);
