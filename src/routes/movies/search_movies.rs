use crate::routes::{movie_db_handler, yts_movie_search_handler};

use super::{
    validate_title, Genre, MovieQuality, SearchOrder, SearchQueryMetadata, SortBy, Source,
};
use actix_web::{
    web::{Data, Json, Query},
    HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

#[derive(Deserialize, Validate, Debug)]
pub struct SearchBody {
    #[validate(custom(function = "validate_title"))]
    pub query_term: String,
    pub genre: Option<Genre>,
    pub source: Option<Source>,
    pub quality: Option<MovieQuality>,
    pub sort_by: Option<SortBy>,
    pub order_by: Option<SearchOrder>,
    pub with_rt_ratings: Option<bool>,
}

#[derive(Deserialize)]
pub struct Paginate {
    pub page_size: Option<u8>,
    pub page: Option<u32>,
}

pub async fn get_movies_search(
    connection: Data<PgPool>,
    body: Json<SearchBody>,
    info: Query<Paginate>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Movie search result");
    let is_valid: Result<(), validator::ValidationErrors> = body.validate();
    if let Err(error) = is_valid {
        let source = error.field_errors();
        for i in source.iter() {
            for err in i.1.iter() {
                if let Some(message) = err.message.as_ref() {
                    tracing::error!("Error: {}", message.as_ref());
                    return HttpResponse::BadRequest().json(json!({
                        "Error" : message.as_ref()
                    }));
                }
            }
        }
        return HttpResponse::BadRequest().finish();
    }

    let search_metadata = SearchQueryMetadata {
        page: info.page.unwrap_or(0u32),
        page_size: info.page_size.unwrap_or(10u8),
        source: body.source.clone().unwrap_or_default(),
        quality: body.quality.clone(),
        query_term: body.query_term.trim().into(),
        genre: body.genre.clone(),
        sort_by: body.sort_by.clone(),
        order_by: body.order_by.clone(),
        with_rt_ratings: body.with_rt_ratings.unwrap_or(false),
    };

    if search_metadata.source == Source::YTS {
        tracing::info!("Calling the YTS Handler");
        let result = yts_movie_search_handler(&connection, query_span.clone(), &search_metadata).await;
        match result {
            Ok(response) => {
                tracing::info!("Got YTS search response");
                return  HttpResponse::Ok().json(json!({
                    "data": response
                }));
            },
            Err(err) => {
                tracing::error!("YTS ERROR");
                return HttpResponse::BadRequest().json(json!({
                    "error": err
                }));
            }
        }
    } else if search_metadata.source == Source::MovieDb {
        tracing::info!("Calling the THE MOVIE DB Handler");
        let result: Result<serde_json::Value, String> = movie_db_handler(&connection, query_span.clone(), &search_metadata).await;
        match result {
            Ok(response) => {
                tracing::info!("Got YTS search response");
                return  HttpResponse::Ok().json(json!({
                    "data": response
                }));
            },
            Err(err) => {
                tracing::error!("YTS ERROR");
                return HttpResponse::BadRequest().json(json!({
                    "error": err
                }));
            }
        }

    }
    HttpResponse::BadRequest().finish()
}
