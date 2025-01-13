CREATE TYPE movie_source_type AS ENUM ('YTS', 'MOVIEDB');

CREATE TABLE watched_movies(
  movie_id INT NOT NULL UNIQUE,
  movie_imdb_code VARCHAR(50),
  user_id uuid NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id),
  movie_source VARCHAR(50) NOT NULL,
  created_at timestamptz NOT NULL
);
