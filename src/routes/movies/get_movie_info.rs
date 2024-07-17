use actix_web::{http, web::Path, HttpResponse};
use serde_json::json;

use super::Source;

pub async fn get_movie_info(path: Path<(u32, Source)>) -> HttpResponse {
    let (movie_id, source_provider) = path.into_inner();

    if source_provider == Source::YTS {
        match yts_api::MovieDetails::new(movie_id)
            .with_cast(true)
            .with_images(true)
            .execute()
            .await
        {
            Ok(res) => {
                return HttpResponse::Ok().json(res);
            }
            Err(err) => {
                return HttpResponse::BadRequest().json(json!({
                  "error": err.to_string()
                }));
            }
        };
    } else if source_provider == Source::MovieDb {
        let client = reqwest::Client::new();
        let movie_db_token = std::env::var("MOVIE_DB_AUTH_TOKEN").unwrap();
        let movie_details = client
            .get(format!(
                "https://api.themoviedb.org/3/movie/{}?language=en-US",
                movie_id
            ))
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", movie_db_token),
            )
            .send()
            .await;
        let response = match movie_details {
            Ok(res) => {
                tracing::info!("Got Movie db search response");
                res
            }
            Err(err) => {
                tracing::error!("MOVIE DB request error {:#?}", err);
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        let mut res_body = match response.json::<serde_json::Value>().await {
            Ok(body) => body,
            Err(err) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        if res_body["backdrop_path"].as_str().is_some() {
            let path = res_body["backdrop_path"].as_str().unwrap();
            *res_body.get_mut("backdrop_path").unwrap() =
                json!(format!("https://image.tmdb.org/t/p/original{}", path));
        }
        if res_body["belongs_to_collection"]["poster_path"]
            .as_str()
            .is_some()
        {
            let path = res_body["belongs_to_collection"]["poster_path"]
                .as_str()
                .unwrap();
            *res_body["belongs_to_collection"]
                .get_mut("poster_path")
                .unwrap() = json!(format!("https://image.tmdb.org/t/p/original{}", path));
        }
        if res_body["belongs_to_collection"]["backdrop_path"]
            .as_str()
            .is_some()
        {
            let path = res_body["belongs_to_collection"]["backdrop_path"]
                .as_str()
                .unwrap();
            *res_body["belongs_to_collection"]
                .get_mut("backdrop_path")
                .unwrap() = json!(format!("https://image.tmdb.org/t/p/original{}", path));
        }
        if res_body["poster_path"].as_str().is_some() {
            let path = res_body["poster_path"].as_str().unwrap();
            *res_body.get_mut("poster_path").unwrap() =
                json!(format!("https://image.tmdb.org/t/p/original{}", path));
        }
        // production_companies // logo_path
        if res_body["production_companies"].as_array_mut().is_some() {
            let arr = res_body["production_companies"].as_array_mut().unwrap();
            for elm in arr.iter_mut() {
                if elm["logo_path"].as_str().is_some() {
                    let path = elm["logo_path"].as_str().unwrap();
                    *elm.get_mut("logo_path").unwrap() =
                        json!(format!("https://image.tmdb.org/t/p/original{}", path));
                }
            }
        }
        // add cast info in response 
        return HttpResponse::Ok().json(res_body);
    }

    HttpResponse::Ok().finish()
}
