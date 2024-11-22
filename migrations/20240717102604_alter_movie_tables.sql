-- Add migration script here
ALTER TABLE favorite_movies
ALTER COLUMN movie_id TYPE VARCHAR(50);

ALTER TABLE watched_movies
ALTER COLUMN movie_id TYPE VARCHAR(50);
