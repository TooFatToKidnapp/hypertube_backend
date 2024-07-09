-- Add migration script here
CREATE TABLE password_verification_code(
	id uuid NOT NULL,
	PRIMARY KEY (id),
	user_id uuid NOT NULL UNIQUE,
	expires_at timestamptz NOT NULL,
	created_at timestamptz NOT NULL,
	code VARCHAR(10) NOT NULL
);
