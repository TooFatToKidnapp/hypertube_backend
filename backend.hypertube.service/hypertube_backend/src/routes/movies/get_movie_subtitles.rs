use super::Source;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use aws_config::imds::client;
// use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct SubtitleInfo {
    pub movie_id: String,
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

    let (id, path) = match sqlx::query(
        r#"
    SELECT * FROM subtiteles WHERE movie_id = $1
    "#,
    )
    .bind(&request_body.movie_id)
    // .bind(request_body.source.clone() as Source)
    .fetch_optional(connection.as_ref())
    .instrument(query_span.clone())
    .await
    {
        Ok(Some(row)) => {
            tracing::info!("Found movie record in the database");
            let subtitle_path = row.get::<String, &str>("path");
            (
                row.get::<Uuid, &str>("id"),
                subtitle_path.split_at(subtitle_path.rfind('/').unwrap()).0.to_string(),
            )
        }
        Ok(None) => {
            let client = reqwest::Client::new();
            let url = format!("https://api.opensubtitles.com/api/v1/subtitles?imdb_id={}", request_body.movie_id);
            let response = client.get("");
            // let SubtitleInfo
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
