use actix_web::{
    web::{Path, Query},
    HttpResponse,
};
use serde_json::json;
use yts_api::{ListMovies, Sort};

use crate::routes::{format_movie_list_response, PageInfo};

use super::Genre;

pub async fn get_yts_top_movies() -> HttpResponse {
    tracing::info!("GETTING YTS TOP 15 MOVIES");

    let mut yts_movie_client = ListMovies::new();

    let res = match yts_movie_client
        .limit(15)
        .sort_by(Sort::Rating)
        .execute()
        .await
    {
        Ok(res) => {
            tracing::info!("Got Top Movie List");
            if res.movies.is_empty() {
                tracing::info!("Top Movie List is empty");
                return HttpResponse::Ok().json(json!({
                  "data": []
                }));
            }
            res
        }
        Err(err) => {
            tracing::error!("Error: getting Movie List {err}");
            return HttpResponse::BadRequest().finish();
        }
    };

    let res = format_movie_list_response(&[], &res);
    HttpResponse::Ok().json(json!({
      "data": res
    }))
}

pub async fn get_yts_top_movies_in_genre(
    genre: Path<Genre>,
    page_info: Query<PageInfo>,
) -> HttpResponse {
    let genre = genre.into_inner().to_string();

    tracing::info!("GETTING YTS TOP MOVIES IN GENRE: {}", genre);

    let page = page_info.page.unwrap_or_default();
    let page_size = if let Some(size) = page_info.page_size {
        if size == 0 {
            10
        } else {
            size
        }
    } else {
        10
    };

    let mut yts_movie_client = ListMovies::new();
    let mut req = yts_movie_client
        .limit(page_size as u8)
        .sort_by(Sort::Rating)
        .genre(&genre);
    if page > 1 {
        req = req.page(page);
    }
    let res = match req.execute().await {
        Ok(res) => {
            tracing::info!("Got Top Movie List");
            if res.movies.is_empty() {
                tracing::info!("Top Movie List is empty");
                return HttpResponse::Ok().json(json!({
                  "data": []
                }));
            }
            res
        }
        Err(err) => {
            tracing::error!("Error: getting Movie List {err}");
            return HttpResponse::BadRequest().finish();
        }
    };

    let res = format_movie_list_response(&[], &res);
    HttpResponse::Ok().json(json!({
      "data": res
    }))
}
