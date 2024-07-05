-- Add migration script here
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
