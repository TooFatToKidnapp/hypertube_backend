-- Add migration script here
CREATE TABLE sessions(
    id uuid NOT NULL UNIQUE,
    user_id uuid NOT NULL,
    session_data json,
    created_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (id)
);
