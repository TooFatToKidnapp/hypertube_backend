use super::{MovieQuality, Source};
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
    pub movie_id: i32,
    pub source: Source,
    // check if the requested quality exist's else start the conversion
    pub quality: MovieQuality,
}

pub async fn stream_video_content(
    connection: Data<PgPool>,
    info: Path<StreamInfo>,
    req: HttpRequest,
) -> HttpResponse {
    let path_info = info.into_inner();
    let query_span = tracing::info_span!("Movie stream handler");

    let query_res = sqlx::query(
        r#"
            SELECT * FROM movie_torrent WHERE movie_id = $1 AND movie_source = $2
        "#,
    )
    .bind(path_info.movie_id)
    .bind(path_info.source.clone() as Source)
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await;

    let movie_path: String = match query_res {
        Ok(torrent_info) => {
            tracing::info!("Got torrent row in database");
            torrent_info.get("movie_path")
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::error!("Torrent Not downloaded");
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            tracing::error!("something went wrong {}", err);
            return HttpResponse::InternalServerError().json(json!({
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
            return HttpResponse::InternalServerError().finish();
        }
    };

    let file_size = metadata.len();

    // Check for Range header
    if let Some(range_header) = req.headers().get(header::RANGE) {
        let range_str = range_header.to_str().unwrap();
        println!("range_str {}", range_str);
        if let Ok(ranges) = HttpRange::parse(range_str, file_size) {
            let first_range = ranges[0];
            let start = first_range.start;
            let end = std::cmp::min(start + first_range.length - 1, file_size - 1);

            let mut buffer = vec![0; (end - start + 1) as usize];
            let mut file = movie;
            file.seek(SeekFrom::Start(start)).unwrap();
            file.read_exact(&mut buffer).unwrap();

            let content_range = ContentRangeSpec::Bytes {
                range: Some((start, end)),
                instance_length: Some(file_size),
            };
            println!("sent chunk size {}", buffer.len());
            println!("sent range {:#?}", (start, end));
            return HttpResponse::PartialContent()
                .insert_header((header::CONTENT_RANGE, content_range))
                .content_type("video/mp4")
                .body(buffer);
        }
    } else {
        let chunk_size = if file_size > 4999999 {
            4999999
        } else {
            file_size
        };
        tracing::info!(
            "No range headers found, sending first {} bytes of the file",
            chunk_size
        );
        if let Ok(ranges) = HttpRange::parse(format!("bytes=0-{}", chunk_size).as_str(), file_size)
        {
            let first_range = ranges[0];
            let start = first_range.start;
            let end = std::cmp::min(start + first_range.length - 1, file_size - 1);

            let mut buffer = vec![0; (end - start + 1) as usize];
            let mut file = movie;
            file.seek(SeekFrom::Start(start)).unwrap();
            file.read_exact(&mut buffer).unwrap();

            let content_range = ContentRangeSpec::Bytes {
                range: Some((start, end)),
                instance_length: Some(file_size),
            };
            println!("sent chunk size {}", buffer.len());
            println!("sent range {:#?}", (start, end));
            return HttpResponse::PartialContent()
                .insert_header((header::CONTENT_RANGE, content_range))
                .content_type("video/mp4")
                .body(buffer);
        }
    }

    todo!()
}
