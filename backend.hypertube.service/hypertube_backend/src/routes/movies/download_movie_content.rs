use crate::routes::{schedule_handler, CronJobScheduler};

use super::torrent::RqbitWrapper;
use super::Source;
use actix_web::web::Json;
use actix_web::{web::Data, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tracing::Instrument; // Import the missing type

#[derive(Deserialize)]
pub struct MovieInfo {
    pub movie_id: String,
    pub source: Source,
    pub magnet_url: String,
}

pub async fn download_torrent(
    connection: Data<PgPool>,
    body: Json<MovieInfo>,
    corn_job_handler: Data<CronJobScheduler>,
) -> HttpResponse {
    let query_span = tracing::trace_span!("Start torrent Download Handler");

    let torrent_client = RqbitWrapper::default();
    // let output_folder = format!("");
    let download_path = {
        let mut base_path = match std::env::current_dir() {
            Ok(dir) => dir.display().to_string(),
            Err(_err) => "/tmp".to_string(),
        };
        base_path.push_str(
            format!(
                "/Download/{}_{}_{}",
                body.movie_id,
                body.source,
                chrono::Utc::now().date_naive()
            )
            .as_str(),
        );
        base_path
    };

    tracing::info!("DOWNLOAD PATH: {}", download_path);

    let meta_data = match torrent_client
        .download_torrent(&body.magnet_url, Some(download_path))
        .await
    {
        Ok(movie_info) => {
            tracing::info!("Got file info {:#?}", movie_info);
            movie_info
        }
        Err(err) => {
            tracing::error!("{}", err);
            return HttpResponse::BadRequest().json(json!({
              "error" : "Failed to start torrent"
            }));
        }
    };

    let query_res = sqlx::query(
    r#"
        INSERT INTO movie_torrent (id, movie_source, movie_id, created_at, movie_path, torrent_id, file_type, available_subs)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    "#)
    .bind(Uuid::new_v4())
    .bind(body.source.clone() as Source)
    .bind(body.movie_id.clone())
    .bind(Utc::now())
    .bind(meta_data.path.clone())
    .bind(meta_data.id.parse::<i32>().unwrap())
    .bind(meta_data.file_type.clone())
    .bind(&meta_data.available_subs)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await;

    match query_res {
        Ok(_) => {
            tracing::info!("torrent created successfully!");
            let _ = schedule_handler(
                &corn_job_handler,
                CronJobScheduler::build_job_id(body.movie_id.clone(), body.source.clone()),
                &connection,
            )
            .await;
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!("Failed to create torrent in database {:#?}", err);
            HttpResponse::BadRequest().finish()
        }
    }
}
