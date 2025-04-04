-- Add migration script here
CREATE TABLE password_verification_code(
	id uuid NOT NULL,
	PRIMARY KEY (id),
	-- user_id uuid NOT NULL UNIQUE,
	username VARCHAR(50) NOT NULL UNIQUE,
	expires_at timestamptz NOT NULL,
	created_at timestamptz NOT NULL,
	code VARCHAR(10) NOT NULL,
	is_validated BOOLEAN DEFAULT FALSE NOT NULL
);
