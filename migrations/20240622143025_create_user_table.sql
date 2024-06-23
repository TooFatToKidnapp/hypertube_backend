-- Add migration script here
CREATE TABLE users(
		id uuid NOT NULL,
		PRIMARY KEY (id),
		username VARCHAR(50) NOT NULL,
		email VARCHAR(50) NOT NULL UNIQUE,
		password VARCHAR(255) NOT NULL,
		created_at timestamptz NOT NULL,
		updated_at timestamptz NOT NULL
);
