CREATE TABLE IF NOT EXISTS Media (
    id           VARCHAR(36) PRIMARY KEY,
    name         TEXT    NOT NULL,
    path         TEXT    NOT NULL UNIQUE,
    alias        TEXT
);
