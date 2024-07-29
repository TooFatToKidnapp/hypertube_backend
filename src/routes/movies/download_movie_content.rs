use super::torrent::RqbitWrapper;
use super::{Source, SubInfo};
use actix_web::web::Json;
use actix_web::{web::Data, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tracing::Instrument; // Import the missing type
use chrono::Utc;
use sqlx::types::Uuid;

#[derive(Deserialize)]
pub struct MovieInfo {
    pub movie_id: u32,
    pub source: Source,
    pub magnet_url: String,
}

pub async fn download_torrent(connection: Data<PgPool>, body: Json<MovieInfo>) -> HttpResponse {
    let query_span = tracing::trace_span!("Start torrent Download Handler");

    let torrent_client = RqbitWrapper::default();
    // let output_folder = format!("");
    let meta_data = match torrent_client
        .download_torrent(&body.magnet_url, None::<String>)
        .await
    {
        Ok(movie_info) => {
            tracing::info!("Got file info {:#?}", movie_info);
            movie_info
        }
        Err(err) => {
            tracing::error!( "{}", err);
            return HttpResponse::InternalServerError().json(json!({
              "error" : "Failed to start torrent"
            }));
        }
    };

    let query_res = sqlx::query!(
      r#"
        INSERT INTO movie_torrent (id, movie_source, movie_id, created_at, movie_path, torrent_id, file_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
      "#,
      Uuid::new_v4(),
      body.source.clone() as Source,
      meta_data.id.parse::<i32>().unwrap(),
      Utc::now(),
      meta_data.path.clone(),
      body.movie_id as i32,
      meta_data.file_type.clone(),
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await;


    match query_res {
        Ok(_) => {
            tracing::info!("torrent created successfully!");
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!("Failed to create torrent in database {:#?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
