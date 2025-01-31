-- Add migration script here
CREATE TABLE users(
		id uuid NOT NULL,
		PRIMARY KEY (id),
		username VARCHAR(50) NOT NULL UNIQUE,
		first_name VARCHAR(50),
		last_name VARCHAR(50),
		profile_picture_url VARCHAR(255),
		email VARCHAR(50) NOT NULL UNIQUE,
		password_hash VARCHAR(255),
		created_at timestamptz NOT NULL,
		updated_at timestamptz NOT NULL,
		finished_profile BOOLEAN DEFAULT FALSE
);
