-- Add migration script here
-- CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

Create Table If Not Exists imdb_movie_details (
    "id"  VARCHAR(15) Not Null,
    "primary_title"   Text,
    "original_title"   Text,
    "source_type"    VARCHAR(15),
    "genres"  Text[],
    "is_adult"    boolean,
    "start_year"   integer,
    "end_year" integer,
    "runtime_minutes"  integer,
    "average_rating"   numeric,
    "num_votes"    integer,
    "description" Text,
    "primary_image"    Text,
    "content_rating"   VARCHAR(15),
    "release_date" Date,
    "interests" Text[],
    "countries_of_origin" Text[],
    "external_links" Text[],
    "spoken_languages" Text[],
    "filming_locations" Text[],
    "directors"   json[],
    "writers" json[],
    "cast"    json[],
    "budget"  numeric,
    "gross_world_wide"  numeric,
    "torrents"    json[]
)