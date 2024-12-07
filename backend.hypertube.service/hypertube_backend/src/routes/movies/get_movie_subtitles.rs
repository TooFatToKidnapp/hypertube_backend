use super::Source;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
// use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct SubtitleInfo {
    pub movie_id: i32,
    pub source: Source,
    pub lang: Option<String>,
}

// https://opensubtitles.stoplight.io/docs/opensubtitles-api/a172317bd5ccc-search-for-subtitles
// async fn download_movie_subtitles(
//     _movie_dir_path: String,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     const SUBTITLES_PROVIDER_URL: &str = "https://api.opensubtitles.com/api/v1/subtitles";
//     let _client = Client::new();
//     // match client.get(SUBTITLES_PROVIDER_URL)
//     // .header("User-Agent", "Hypertube v0.1")

//     todo!()
// }

pub async fn get_movie_subtitles(
    connection: Data<PgPool>,
    body: Json<SubtitleInfo>,
) -> HttpResponse {
    let request_body = body.into_inner();

    let query_span = tracing::info_span!("Get Movie Subtitles Handler");

    let (_movie_record_id, _movie_dir_path) = match sqlx::query(
        r#"
    SELECT * FROM movie_torrent WHERE movie_id = $1 AND movie_source = $2
    "#,
    )
    .bind(request_body.movie_id)
    .bind(request_body.source.clone() as Source)
    .fetch_optional(connection.as_ref())
    .instrument(query_span.clone())
    .await
    {
        Ok(Some(row)) => {
            tracing::info!("Found movie record in the database");
            let path = row.get::<String, &str>("movie_path");
            (
                row.get::<Uuid, &str>("id"),
                path.split_at(path.rfind('/').unwrap()).0.to_string(),
            )
        }
        Ok(None) => {
            tracing::info!("Movie record not found");
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
              "error" : "Something went wrong"
            }));
        }
    };

    todo!()
}
