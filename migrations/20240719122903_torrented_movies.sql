-- Add migration script here
CREATE TABLE movie_torrent(
  id uuid NOT NULL,
  PRIMARY KEY (id),
  movie_source movie_source_type NOT NULL,
  movie_id INT NOT NULL,
  movie_imdb_code VARCHAR(50),
  created_at timestamptz NOT NULL,
  torrent_id INT NOT NULL
);
