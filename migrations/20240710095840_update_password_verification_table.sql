-- Add migration script here
ALTER TABLE password_verification_code
ADD COLUMN is_validated BOOLEAN DEFAULT FALSE;
