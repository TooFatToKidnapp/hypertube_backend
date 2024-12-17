use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;
use sqlx;

#[derive(Deserialize, Debug, Default, PartialEq, Clone, sqlx::Type)]
pub struct ImdbMovieDetails {
    pub id: String,
    pub primary_title: String,
    pub original_title: String,
    pub source_type: String,
    pub genres: Vec<String>,
    pub is_adult: bool,
    pub start_year: i32,
    pub end_year: i32,
    pub runtime_minutes: i32,
    pub average_rating: f32,
    pub num_votes: i32,
    pub description: String,
    pub primary_image: String,
    pub content_rating: String,
    pub release_date: Option<NaiveDate>,
    pub interests: Vec<String>,
    pub countries_of_origin: Vec<String>,
    pub external_links: Vec<String>,
    pub spoken_languages: Vec<String>,
    pub filming_locations: Vec<String>,
    pub directors: Vec<Value>,
    pub writers: Vec<Value>,
    pub cast: Vec<Value>,
    pub budget: f64,
    pub gross_world_wide: f64,
    pub torrents: Vec<Value>,
}
