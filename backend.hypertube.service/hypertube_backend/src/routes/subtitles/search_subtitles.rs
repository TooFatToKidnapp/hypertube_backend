use actix_web::{web::{Data, Path}, HttpRequest, HttpResponse};
use serde_json::json;
use serde::Deserialize;
use sqlx::PgPool;
use validator::Validate;

async fn fetch_subtitles_search(imdb_id: &String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

    let opensubtitles_url = match std::env::var("OPENSUBTITLE_ENDPOINT") {
        Ok(url) => url,
        Err(err) => return Err(format!("Failed to get OPENSUBTITLE_ENDPOINT: {}", err)),
    };

    let opensubtitles_api_key = match std::env::var("OPENSUBTITLE_API_KEY") {
        Ok(key) => key,
        Err(err) => return Err(format!("Failed to get OPENSUBTITLE_API_KEY: {}", err)),
    };

    let search_url = format!("https://{}/subtitles?imdb_id={}", opensubtitles_url, imdb_id);

    let search_subtitle_res = client
        .get(&search_url)
        .header("Api-Key", opensubtitles_api_key)
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

#[derive(Deserialize)]
pub struct RequestParam {
    pub imdb_id: String,
}

pub async fn get_subtiles_search(
    req: HttpRequest,
    connection: Data<PgPool>,
    path: Path<RequestParam>,

) -> HttpResponse {
    let parsed_imdb_id = match path.imdb_id.parse::<String>(){
        Ok(imdb_id) => imdb_id,
        Err(err) => {
            tracing::error!("Error parsing param id {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Error parsing param id"
            }));
        }
    };

    match fetch_subtitles_search(&parsed_imdb_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            tracing::error!("Error fetching subtitles: {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "Error fetching subtitles"
            }))
        }
    }

}