use super::validate_title;
use actix_web::{
    web::{Data, Json, Query},
    HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::fmt;
use tracing::{Instrument, Span};
use uuid::Uuid;
use validator::Validate;
use yts_api::{ListMovies, MovieList};

// https://ww4.yts.nz/api
// https://popcornofficial.docs.apiary.io/#reference/show/get-page/pages
// https://crates.io/crates/yts-api
// https://developer.themoviedb.org/docs/getting-started

#[derive(Deserialize, Debug, Clone)]
pub enum Genre {
    Action,
    Drama,
    Thriller,
    Crime,
    Adventure,
    Comedy,
    SciFi,
    Romance,
    Fantasy,
    Horror,
    Mystery,
    War,
    Animation,
    History,
    Family,
    Western,
    Sport,
    Documentary,
    Biography,
    Musical,
    Music,
    FilmNoir,
    News,
    RealityTV,
    GameShow,
    TalkShow,
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Genre::Action => "action",
                Genre::Adventure => "adventure",
                Genre::Animation => "animation",
                Genre::Biography => "biography",
                Genre::Crime => "crime",
                Genre::Comedy => "comedy",
                Genre::Drama => "drama",
                Genre::Documentary => "documentary",
                Genre::Family => "family",
                Genre::Fantasy => "fantasy",
                Genre::FilmNoir => "film-noir",
                Genre::GameShow => "game-show",
                Genre::History => "history",
                Genre::Horror => "horror",
                Genre::Musical => "musical",
                Genre::Music => "music",
                Genre::Mystery => "mystery",
                Genre::News => "news",
                Genre::RealityTV => "reality-tv",
                Genre::Romance => "romance",
                Genre::Sport => "sport",
                Genre::SciFi => "sci-fi",
                Genre::TalkShow => "talk-show",
                Genre::Thriller => "thriller",
                Genre::War => "war",
                Genre::Western => "western",
            }
        )
    }
}

#[derive(Deserialize, Debug, Default, PartialEq, Clone, sqlx::Type)]
#[sqlx(type_name = "movie_source_type", rename_all = "UPPERCASE")]
pub enum Source {
    #[default]
    YTS,
    PopcornOfficial,
    MovieDb,
}

#[derive(Deserialize, Validate, Debug)]
pub struct SearchBody {
    #[validate(custom(function = "validate_title"))]
    pub query_term: String,
    pub genre: Option<Genre>,
    pub source: Option<Source>,
    pub quality: Option<String>,
    minimum_rating: Option<u8>,
    sort_by: Option<SortBy>,
    order_by: Option<SearchOrder>,
    with_rt_ratings: Option<bool>,
}

#[derive(Deserialize)]
pub struct Paginate {
    pub page_size: Option<u8>,
    pub page: Option<u32>,
}

#[derive(Default, Deserialize, Debug, Clone)]
pub enum SearchOrder {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug)]
struct SearchQueryMetadata {
    pub page: u32,
    pub page_size: u8,
    pub source: Source,
    pub quality: Option<String>,
    pub minimum_rating: Option<u8>,
    pub query_term: String,
    pub genre: Option<Genre>,
    pub sort_by: Option<SortBy>,
    pub order_by: Option<SearchOrder>,
    pub with_rt_ratings: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub enum SortBy {
    Title,
    Year,
    Rating,
    Peers,
    Seeds,
    #[allow(non_camel_case_types)]
    Download_count,
    #[allow(non_camel_case_types)]
    Like_count,
    #[allow(non_camel_case_types)]
    Sate_added,
}

// YTS PAGE cant be <= 1
//  1 < page_size <= 50
// title must not contain white space, replace space with '-'

async fn query_yts_content_provider(
    metadata: &SearchQueryMetadata,
) -> Result<MovieList, Box<dyn std::error::Error + Send + Sync>> {
    let mut yts_movie_client = ListMovies::new();
    let mut res = yts_movie_client
        .limit(metadata.page_size)
        .query_term(&metadata.query_term);
    if metadata.page > 1 {
        res = res.page(metadata.page)
    }
    let res = res.execute().await?;
    Ok(res)
}

fn format_movie_list_response(ids: &Vec<String>, list: &MovieList) -> serde_json::Value {
    let mut res = json!({
        "data": {
            "limit": list.limit,
            "movie_count": list.movie_count,
            "movies": []
        }
    });

    let arr = res["data"]["movies"].as_array_mut().unwrap();

    for movie in list.movies.iter() {
        let mut movie_content = json!({
            "genres": movie.genres,
            "id": movie.id,
            "imdb_code": movie.imdb_code,
            "language": movie.language,
            "large_cover_image": movie.large_cover_image,
            "medium_cover_image": movie.medium_cover_image,
            "small_cover_image": movie.small_cover_image,
            "summary": movie.summary,
            "synopsis": movie.synopsis,
            "title": movie.title,
            "title_english": movie.title_english,
            "title_long": movie.title_long,
            "year": movie.year,
            "rating": movie.rating,
            "watched": false
        });

        if ids.contains(&movie.imdb_code) {
            movie_content["watched"] = json!(true);
        }
        arr.push(movie_content);
    }
    res
}

async fn get_watched_movies_ids(
    list: &MovieList,
    connection: &Data<PgPool>,
    span: Span,
) -> Result<Vec<String>, sqlx::Error> {
    let movies_imdb_ids = list
        .movies
        .iter()
        .map(|movie| format!("'{}'", movie.imdb_code))
        .collect::<Vec<_>>();
    let query = format!(
        "SELECT * FROM watched_movies WHERE movie_imdb_code IN ({})",
        movies_imdb_ids.join(", ")
    );
    let query_res = sqlx::query(query.as_str())
        .fetch_all(connection.as_ref())
        .instrument(span)
        .await;

    if let Err(err) = query_res {
        return Err(err);
    }

    let data = query_res.unwrap();

    let ids = data
        .iter()
        .map(|row| row.get::<String, &str>("movie_imdb_code"))
        .collect();
    Ok(ids)
}

pub async fn get_movie_list(
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
        minimum_rating: body.minimum_rating,
        query_term: body.query_term.trim().into(),
        genre: body.genre.clone(),
        sort_by: body.sort_by.clone(),
        order_by: body.order_by.clone(),
        with_rt_ratings: body.with_rt_ratings.unwrap_or(false),
    };

    if search_metadata.source == Source::YTS {
        tracing::info!("Calling the YTS Handler");
        let yts_res = query_yts_content_provider(&search_metadata).await;
        let movie_list = match yts_res {
            Ok(list) => list,
            Err(err) => {
                tracing::error!("YTS HANDLER ERR {:#?}", err);
                return HttpResponse::InternalServerError().json(json!({
                    "error": err.to_string()
                }));
            }
        };

        let watched_movies_ids_res =
            get_watched_movies_ids(&movie_list, &connection, query_span.clone()).await;

        let ids = match watched_movies_ids_res {
            Ok(ids) => {
                tracing::info!("Got Watched Movies ids");
                ids
            }
            Err(err) => {
                tracing::error!("Database Error {:#?}", err);
                return HttpResponse::InternalServerError().json(json!({
                    "error": "Something went wrong"
                }));
            }
        };

        return HttpResponse::Ok().json(json!({
            "data": movie_list
        }));
        return HttpResponse::Ok().json(movie_list);
    }
    HttpResponse::Ok().finish()
}
