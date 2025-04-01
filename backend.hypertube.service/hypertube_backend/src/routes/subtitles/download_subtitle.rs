use actix_web::{web::{Data, Path}, HttpRequest, HttpResponse};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tokio::fs::{create_dir_all, File as TokioFile};
use tokio::io::AsyncWriteExt;
use validator::Validate;

#[derive(Deserialize)]
pub struct RequestParam {
    pub file_id: String,
}

async fn fetch_subtitles_download(file_id: &String) -> Result<serde_json::Value, String> {
    let client = Client::new();

    let opensubtitles_url = match std::env::var("OPENSUBTITLE_ENDPOINT") {
        Ok(url) => url,
        Err(err) => return Err(format!("Failed to get OPENSUBTITLE_ENDPOINT: {}", err)),
    };

    let opensubtitles_api_key = match std::env::var("OPENSUBTITLE_API_KEY") {
        Ok(key) => key,
        Err(err) => return Err(format!("Failed to get OPENSUBTITLE_API_KEY: {}", err)),
    };

    let search_url = format!("https://{}/download?file_id={}", opensubtitles_url, file_id);

    let search_subtitle_res = client
        .post(&search_url)
        .header("Api-Key", opensubtitles_api_key)
        .json(&json!({"file_id": file_id}))
        .send()
        .await;

    let response = match search_subtitle_res {
        Ok(val) => {
            tracing::info!("Got SUBTITLE search response");
            tracing::info!("SUBTITLE search response:: {:#?}", val);
            val
        }
        Err(err) => {
            tracing::error!("SEARCH SUBTITLE ERROR : {:#?}", err);
            return Err(err.to_string());
        }
    };

    if response.status() == 429 {
        return Err(String::from("you exceeded your daily QUOTA"));
    }

    let response_body = response.json::<serde_json::Value>().await;

    let res = match response_body {
        Ok(val) => {
            tracing::info!("SEARCh body:::: {:#?}", val);
            val
        }
        Err(err) => {
            tracing::error!("Failed to get search response body");
            return Err(err.to_string());
        }
    };
    Ok(res)
}

pub async fn download_subtile_file(
    req: HttpRequest,
    connection: Data<PgPool>,
    path: Path<RequestParam>,
) -> HttpResponse {
    let parsed_file_id = match path.file_id.parse::<String>() {
        Ok(file_id) => file_id,
        Err(err) => {
            tracing::error!("Error parsing param id {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Error parsing param id"
            }));
        }
    };

    // Check if the subtitle already exists in the database
    let existing_subtitle = sqlx::query!(
        "SELECT file_name, content FROM subtitles WHERE file_id = $1",
        parsed_file_id
    )
    .fetch_optional(connection.get_ref())
    .await;

    match existing_subtitle {
        Ok(Some(record)) => {
            // Serve the existing subtitle
            HttpResponse::Ok().json(json!({
                "message": "Subtitle served from database",
                "file_name": record.file_name,
                "content": String::from_utf8_lossy(&record.content)
            }))
        }
        Ok(None) => {
            // Download the subtitle if it doesn't exist in the database
            match fetch_subtitles_download(&parsed_file_id).await {
                Ok(result) => {
                    let link = result["link"].as_str().unwrap_or("");
                    let file_name = result["file_name"].as_str().unwrap_or("subtitle.srt");

                    let client = Client::new();
                    let subtitle_res = client.get(link).send().await;
                    match subtitle_res {
                        Ok(subtitle_response) => {
                            let content = subtitle_response.bytes().await.unwrap();

                            // Store subtitle in the database
                            let query = sqlx::query!(
                                "INSERT INTO subtitles (file_id, file_name, content) VALUES ($1, $2, $3)",
                                parsed_file_id,
                                file_name,
                                content.to_vec()
                            );
                            match query.execute(connection.get_ref()).await {
                                Ok(_) => {
                                    HttpResponse::Ok().json(json!({
                                        "message": "Subtitle downloaded and stored successfully",
                                        "file_name": file_name,
                                        "content": String::from_utf8_lossy(&content)
                                    }))
                                }
                                Err(err) => {
                                    tracing::error!("Error storing subtitle in database: {:#?}", err);
                                    HttpResponse::InternalServerError().json(json!({
                                        "error": "Error storing subtitle in database"
                                    }))
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!("Error downloading subtitle: {:#?}", err);
                            HttpResponse::InternalServerError().json(json!({
                                "error": "Error downloading subtitle"
                            }))
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("Error fetching subtitles: {:#?}", err);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "Error fetching subtitles"
                    }))
                }
            }
        }
        Err(err) => {
            tracing::error!("Error querying database: {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "Error querying database"
            }))
        }
    }
}


// pub async fn download_subtile_file(
//     req: HttpRequest,
//     connection: Data<PgPool>,
//     path: Path<RequestParam>,
// ) -> HttpResponse {
//     let parsed_file_id = match path.file_id.parse::<String>() {
//         Ok(file_id) => file_id,
//         Err(err) => {
//             tracing::error!("Error parsing param id {:#?}", err);
//             return HttpResponse::BadRequest().json(json!({
//                 "error": "Error parsing param id"
//             }));
//         }
//     };

//     // Check if the subtitle already exists in the database
//     let existing_subtitle = sqlx::query!(
//         "SELECT file_name, content FROM subtitles WHERE file_id = $1",
//         parsed_file_id
//     )
//     .fetch_optional(connection.get_ref())
//     .await;

//     match existing_subtitle {
//         Ok(Some(record)) => {
//             // Serve the existing subtitle
//             HttpResponse::Ok().json(json!({
//                 "message": "Subtitle served from database",
//                 "file_name": record.file_name,
//                 "content": String::from_utf8_lossy(&record.content)
//             }))
//         }
//         Ok(None) => {
//             // Download the subtitle if it doesn't exist in the database
//             match fetch_subtitles_download(&parsed_file_id).await {
//                 Ok(result) => {
//                     let link = result["link"].as_str().unwrap_or("");
//                     let file_name = result["file_name"].as_str().unwrap_or("subtitle.srt");

//                     let client = Client::new();
//                     let subtitle_res = client.get(link).send().await;
//                     match subtitle_res {
//                         Ok(subtitle_response) => {
//                             let content = subtitle_response.bytes().await.unwrap();

//                             // Create directories
//                             let dir_path = format!("subtitles/{}", parsed_file_id);
//                             create_dir_all(&dir_path).await.unwrap();

//                             // Save the subtitle file
//                             let file_path = format!("{}/{}", dir_path, file_name);
//                             let mut file = TokioFile::create(&file_path).await.unwrap();
//                             file.write_all(&content).await.unwrap();

//                             // Store subtitle in the database
//                             let query = sqlx::query!(
//                                 "INSERT INTO subtitles (file_id, file_name, content) VALUES ($1, $2, $3)",
//                                 parsed_file_id,
//                                 file_name,
//                                 content.to_vec()
//                             );
//                             match query.execute(connection.get_ref()).await {
//                                 Ok(_) => {
//                                     HttpResponse::Ok().json(json!({
//                                         "message": "Subtitle downloaded and stored successfully",
//                                         "file_name": file_name,
//                                         "content": String::from_utf8_lossy(&content)
//                                     }))
//                                 }
//                                 Err(err) => {
//                                     tracing::error!("Error storing subtitle in database: {:#?}", err);
//                                     HttpResponse::InternalServerError().json(json!({
//                                         "error": "Error storing subtitle in database"
//                                     }))
//                                 }
//                             }
//                         }
//                         Err(err) => {
//                             tracing::error!("Error downloading subtitle: {:#?}", err);
//                             HttpResponse::InternalServerError().json(json!({
//                                 "error": "Error downloading subtitle"
//                             }))
//                         }
//                     }
//                 }
//                 Err(err) => {
//                     tracing::error!("Error fetching subtitles: {:#?}", err);
//                     HttpResponse::InternalServerError().json(json!({
//                         "error": "Error fetching subtitles"
//                     }))
//                 }
//             }
//         }
//         Err(err) => {
//             tracing::error!("Error querying database: {:#?}", err);
//             HttpResponse::InternalServerError().json(json!({
//                 "error": "Error querying database"
//             }))
//         }
//     }
// }
