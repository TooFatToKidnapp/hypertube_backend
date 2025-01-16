CREATE TABLE favorite_movies (
  user_id uuid NOT NULL,
  poster_src TEXT NOT NULL,
  title TEXT NOT NULL,
  movie_id INT NOT NULL,
  movie_imdb_code VARCHAR(50),
  movie_source VARCHAR(50) NOT NULL,
  created_at timestamptz NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id),
  UNIQUE (user_id, movie_id)
);