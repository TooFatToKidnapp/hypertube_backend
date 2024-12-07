-- Add migration script here
CREATE TABLE movie_torrent(
  id uuid NOT NULL,
  PRIMARY KEY (id),
  movie_source movie_source_type NOT NULL,
  movie_id INT NOT NULL,
  created_at timestamptz NOT NULL,
  movie_path TEXT NOT NULL,
  file_type TEXT NOT NULL,
  available_subs JSONB[],
  torrent_id INT NOT NULL
);
