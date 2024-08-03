-- Add migration script here
CREATE TABLE user_comments(
  id uuid NOT NULL,
  movie_source movie_source_type NOT NULL,
  movie_id INT NOT NULL,
  created_at timestamptz NOT NULL,
  user_id uuid NOT NULL,
  comment TEXT NOT NULL
);
