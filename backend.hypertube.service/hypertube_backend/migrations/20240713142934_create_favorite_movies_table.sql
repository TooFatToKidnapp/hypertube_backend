CREATE TABLE favorite_movies (
  user_id uuid NOT NULL,
  movie_id INT NOT NULL,
  movie_imdb_code VARCHAR(50),
  movie_source movie_source_type NOT NULL,
  created_at timestamptz NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id)
);
