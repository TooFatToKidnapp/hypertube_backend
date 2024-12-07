use actix_web::{http, web::Path, HttpResponse};
use serde_json::json;

use super::{map_movie_bd_genre_code_with_value, Source};
// https://trakt.tv
// https://trakt.docs.apiary.io/#introduction/standard-media-objects
pub async fn get_movie_info(path: Path<(u32, Source)>) -> HttpResponse {
    let (movie_id, source_provider) = path.into_inner();

    if source_provider == Source::YTS {
        let movie_details = match yts_api::MovieDetails::new(movie_id)
            .with_cast(true)
            .with_images(true)
            .execute()
            .await
        {
            Ok(mut res) => {
                res.movie.yt_trailer_code = format!(
                    "https://www.youtube.com/watch?v={}",
                    res.movie.yt_trailer_code
                );
                res
            }
            Err(err) => {
                return HttpResponse::BadRequest().json(json!({
                  "error": err.to_string()
                }));
            }
        };
        let client = reqwest::Client::new();
        let similar_movies_suggestions = match client
            .get(format!(
                "https://yts.mx/api/v2/movie_suggestions.json?movie_id={}",
                movie_id
            ))
            .send()
            .await
        {
            Ok(res) => {
                tracing::info!("Got YTS movie recommendations");
                res.json::<serde_json::Value>().await
            }
            Err(err) => {
                tracing::error!("YTS movie recommendations Error");
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        match similar_movies_suggestions {
            Ok(res) => {
                return HttpResponse::Ok().json(json!({
                    "data": movie_details,
                    "movie_suggestions" : res
                }));
            }
            Err(_) => {
                return HttpResponse::Ok().json(json!({
                    "data": movie_details,
                }));
            }
        }
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
        let cast_info = match client
            .get(format!(
                "https://api.themoviedb.org/3/movie/{}/credits?language=en-US",
                movie_id
            ))
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", movie_db_token),
            )
            .send()
            .await
        {
            Ok(cast_res) => cast_res,
            Err(err) => {
                tracing::error!("Movie Db error getting cast {:#?}", err);
                return HttpResponse::BadRequest().json(json!({
                    "error": "error getting movie cast"
                }));
            }
        };
        if let Ok(mut cast_body) = cast_info.json::<serde_json::Value>().await {
            if cast_body["cast"].as_array().is_some() {
                let cast_arr_ref = cast_body["cast"].as_array_mut().unwrap();
                for cast in cast_arr_ref.iter_mut() {
                    if cast["profile_path"].as_str().is_some() {
                        let path = cast["profile_path"].as_str().unwrap();
                        *cast.get_mut("profile_path").unwrap() =
                            json!(format!("https://image.tmdb.org/t/p/w500{}", path));
                    }
                }
                res_body["cast"] = cast_body["cast"].clone();
            }
        }

        let trailer_response = match client
            .get(format!(
                "https://api.themoviedb.org/3/movie/{}/videos?language=en-US",
                movie_id
            ))
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", movie_db_token),
            )
            .send()
            .await
        {
            Ok(res) => {
                tracing::info!("Got Movie db trailer response");
                res.json::<serde_json::Value>().await
            }
            Err(err) => {
                tracing::error!("The Movie Db Movie Trailer error {:#?}", err);
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        if let Ok(mut trailer_body) = trailer_response {
            if trailer_body["results"].as_array().is_some() {
                let trailer_arr = trailer_body["results"].as_array_mut().unwrap();
                for elem in trailer_arr.iter_mut() {
                    if elem["key"].as_str().is_some() {
                        let key = elem["key"].as_str().unwrap();
                        elem["trailer_url"] =
                            json!(format!("https://www.youtube.com/watch?v={}", key));
                    }
                }
            }
            res_body["trailer_info"] = trailer_body["results"].clone();
        };

        let recommendation_response = match client
            .get(format!(
                "https://api.themoviedb.org/3/movie/{}/similar?language=en-US&page=1",
                movie_id
            ))
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", movie_db_token),
            )
            .send()
            .await
        {
            Ok(res) => {
                tracing::info!("Got Movie db recommendation response");
                res.json::<serde_json::Value>().await
            }
            Err(err) => {
                tracing::error!("The Movie Db Movie recommendation error {:#?}", err);
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        if let Ok(mut rec_body) = recommendation_response {
            if rec_body["results"].as_array().is_some() {
                let trailer_arr = rec_body["results"].as_array_mut().unwrap();
                for elem in trailer_arr.iter_mut() {
                    if elem["backdrop_path"].as_str().is_some() {
                        let path = elem["backdrop_path"].as_str().unwrap();
                        *elem.get_mut("backdrop_path").unwrap() =
                            json!(format!("https://image.tmdb.org/t/p/original{}", path));
                    }
                    if elem["poster_path"].as_str().is_some() {
                        let path = elem["poster_path"].as_str().unwrap();
                        *elem.get_mut("poster_path").unwrap() =
                            json!(format!("https://image.tmdb.org/t/p/w500{}", path));
                    }
                    if elem["genre_ids"].as_array().is_some() {
                        let genre_arr = elem["genre_ids"].as_array().unwrap();
                        let value_arr = genre_arr
                            .iter()
                            .map(|g| json!(g))
                            .collect::<Vec<serde_json::Value>>();
                        let genres = map_movie_bd_genre_code_with_value(&value_arr);
                        *elem.get_mut("genre_ids").unwrap() = json!(genres);
                    }
                }
            }
            res_body["similar_movies"] = rec_body["results"].clone();
        };

        return HttpResponse::Ok().json(res_body);
    }
    HttpResponse::BadRequest().finish()
}
