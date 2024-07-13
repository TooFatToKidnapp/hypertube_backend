CREATE TYPE movie_source_type AS ENUM ('YTS', 'POPCORN_OFFICIAL', 'MOVIE_DB');

CREATE TABLE watched_movies(
	movie_id INT NOT NULL,
	user_id uuid NOT NULL,
	FOREIGN KEY (user_id) REFERENCES users(user_id),
	movie_source movie_source_type NOT NULL,
  created_at timestamptz NOT NULL,
)
