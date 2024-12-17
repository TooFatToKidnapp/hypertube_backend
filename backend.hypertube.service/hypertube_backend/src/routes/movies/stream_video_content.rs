use crate::routes::{cancel_job, schedule_handler};

use super::{CronJobScheduler, MovieQuality, Source};
use actix_files::HttpRange;
use actix_web::{
    http::header::{self, ContentRangeSpec},
    web::{Data, Path},
    HttpRequest, HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use tracing::Instrument;

#[derive(Deserialize)]
pub struct StreamInfo {
    pub movie_id: String,
    pub source: Source,
    // check if the requested quality exist's else start the conversion
    pub _quality: MovieQuality,
}

pub async fn stream_video_content(
    connection: Data<PgPool>,
    info: Path<StreamInfo>,
    req: HttpRequest,
    mut corn_job_handler: Data<CronJobScheduler>,
) -> HttpResponse {
    let path_info = info.into_inner();
    let query_span = tracing::info_span!("Movie stream handler");

    let query_res = sqlx::query(
        r#"
            SELECT * FROM movie_torrent WHERE movie_id = $1 AND movie_source = $2
        "#,
    )
    .bind(path_info.movie_id.clone())
    .bind(path_info.source.clone() as Source)
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await;

    let (movie_path, file_type) = match query_res {
        Ok(torrent_info) => {
            tracing::info!("Got torrent row in database");
            (
                torrent_info.get::<&str, &str>("movie_path").to_string(),
                torrent_info.get::<&str, &str>("file_type").to_string(),
            )
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::error!("Torrent Not downloaded");
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            tracing::error!("something went wrong {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Database error"
            }));
        }
    };

    let movie = match File::open(movie_path.clone()) {
        Ok(file) => {
            tracing::info!("Opened file!");
            file
        }
        Err(err) => {
            tracing::error!("Can't Open file {}", err);
            tracing::warn!("cant find file: {}", movie_path);
            return HttpResponse::NotFound().json(json!({
                "error": "File not found"
            }));
        }
    };

    // get file metadata
    let metadata = match movie.metadata() {
        Ok(metadata) => {
            tracing::info!("Got file metadata");
            metadata
        }
        Err(err) => {
            tracing::error!("Can't get file data {}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    // set movie as watched
    let _ = cancel_job(
        &mut corn_job_handler,
        CronJobScheduler::build_job_id(path_info.movie_id.clone(), path_info.source.clone()),
    )
    .await;
    let _ = schedule_handler(
        &corn_job_handler,
        CronJobScheduler::build_job_id(path_info.movie_id.clone(), path_info.source.clone()),
        &connection,
    )
    .await;

    let file_size = metadata.len();
    // Check for Range header
    if let Some(range_header) = req.headers().get(header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            if range_str.is_empty() {
                tracing::error!("empty range header");
                return HttpResponse::BadRequest().json(json!({
                    "error": "invalid range header"
                }));
            }
            if let Ok(ranges) = HttpRange::parse(range_str, file_size) {
                const CHUNK_SIZE_3_MB: u64 = 3_145_728;
                let first_range = ranges[0];
                let start = first_range.start;
                let end =
                    std::cmp::min(start + first_range.length - 1, start + CHUNK_SIZE_3_MB - 1);
                let mut buffer = vec![0; (end - start + 1) as usize];
                let mut file = movie;
                file.seek(SeekFrom::Start(start)).unwrap();
                file.read_exact(&mut buffer).unwrap();

                let content_range = ContentRangeSpec::Bytes {
                    range: Some((start, end)),
                    instance_length: Some(file_size),
                };
                return HttpResponse::PartialContent()
                    .insert_header((header::CONTENT_RANGE, content_range))
                    .content_type(format!("video/{}", file_type))
                    .body(buffer);
            } else {
                tracing::error!("Cant parse the range header");
                HttpResponse::BadRequest().json(json!({
                    "error": "invalid range header"
                }))
            }
        } else {
            tracing::error!("Invalid header value format");
            HttpResponse::BadRequest().json(json!({
                "error": "invalid range header"
            }))
        }
    } else {
        tracing::error!("Missing range header in the request");
        HttpResponse::BadRequest().json(json!({
            "error" : "Missing range header"
        }))
    }
}
