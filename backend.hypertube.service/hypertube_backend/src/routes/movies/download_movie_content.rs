use std::process::Command;

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


fn convert_video(input_path: &str, output_path: &str, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Convert the video using FFmpeg
    let output_file = format!("{}.{}", output_path, format);
    Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg(&output_file)
        .status()?;
    Ok(())
}

fn get_download_folder() -> Result<PathBuf, String> {
    let current_dir = env::current_dir().map_err(|err| format!("failed to get current directory{}", err))?;
    let parent_dir = current_dir.parent().ok_or("Failed to get parrent Directory")?;
    let target_folder = parent_dir.join("Downloads");

    Ok(target_folder)
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
        // let mut base_path = match std::env::current_dir() {
        //     Ok(dir) => dir.display().to_string(),
        //     Err(_err) => "/tmp".to_string(),
        // };
        let path = format!("{}_{}_{}",body.movie_id, body.source, chrono::Utc::now().date_naive());
        path
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

    if (meta_data.file_type != "mp4" || meta_data.file_type != "webm"){
        tracing::info!("--------HERE START CONVERT TO MKV---------");
        let converted_path = format!("{}/{}",get_download_folder(), "videos/converted/video");
        convert_video(&meta_data.path, &converted_path, "mkv");
        convert_video(&meta_data.path, &converted_path, "m3u8");

        // let client = reqwest::Client::new();
        // let res = client.post("localhost:3001/hls")
        //                             .json(&json!({
        //                                 "video_path": meta_data.path,
        //                                 "id": body.movie_id
        //                             })).send().await;
        // let response = match res {
        //     Ok(val) => {
        //         val
        //     }
        //     Err(err) => {
        //         // use the error response from the server
        //         return  HttpResponse::InternalServerError().json(json!({"error":"couldn't convert file"}));
        //     }
        // };

        tracing::info!("--------HERE END CONVERT TO MKV---------");
    }

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
