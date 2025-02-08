-- Add migration script here
CREATE TABLE subtitles (
    id SERIAL PRIMARY KEY,
    file_id VARCHAR NOT NULL,
    file_name VARCHAR NOT NULL,
    content BYTEA NOT NULL
);