use super::get_movies_search;
use crate::middleware::Authentication;
use actix_web::web::Data;
use actix_web::{web, Scope};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::borrow::Cow;
use std::fmt;
use tracing::{Instrument, Span};
use validator::ValidationError;
use yts_api::{ListMovies, MovieList, Order, Quality, Sort};

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
                Genre::Action => "Action",
                Genre::Adventure => "Adventure",
                Genre::Animation => "Animation",
                Genre::Biography => "Biography",
                Genre::Crime => "Crime",
                Genre::Comedy => "Comedy",
                Genre::Drama => "Drama",
                Genre::Documentary => "Documentary",
                Genre::Family => "Family",
                Genre::Fantasy => "Fantasy",
                Genre::FilmNoir => "Film-Noir",
                Genre::GameShow => "Game-Show",
                Genre::History => "History",
                Genre::Horror => "Horror",
                Genre::Musical => "Musical",
                Genre::Music => "Music",
                Genre::Mystery => "Mystery",
                Genre::News => "News",
                Genre::RealityTV => "Reality-Tv",
                Genre::Romance => "Romance",
                Genre::Sport => "Sport",
                Genre::SciFi => "Sci-Fi",
                Genre::TalkShow => "Talk-Show",
                Genre::Thriller => "Thriller",
                Genre::War => "War",
                Genre::Western => "Western",
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

#[derive(Deserialize, Copy, Clone, Debug)]
pub enum MovieQuality {
    Q720p,
    Q1080p,
    Q2160p,
    Q3D,
}

#[derive(Default, Deserialize, Debug, Clone)]
pub enum SearchOrder {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug)]
pub struct SearchQueryMetadata {
    pub page: u32,
    pub page_size: u8,
    pub source: Source,
    pub quality: Option<MovieQuality>,
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
    DownloadCount,
    LikeCount,
    DateAdded,
}

pub fn movie_source(db_pool: &PgPool) -> Scope {
    web::scope("/movies").route(
        "/search",
        web::post()
            .to(get_movies_search)
            .wrap(Authentication::new(db_pool.clone())),
    )
}

pub fn validate_title(str: &str) -> Result<(), ValidationError> {
    if str.trim().is_empty() {
        return Err(
            ValidationError::new("Invalid length").with_message(Cow::from("title Can't be empty"))
        );
    }

    Ok(())
}

pub async fn get_watched_movies_ids(
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

pub fn format_movie_list_response(ids: &Vec<String>, list: &MovieList) -> serde_json::Value {
    let mut res = json!({
            "limit": list.limit,
            "max_movie_count": list.movie_count,
            "max_page_count": (list.movie_count / list.limit).checked_sub(1).unwrap_or(1)  ,
            "movies": []
    });

    let arr = res["movies"].as_array_mut().unwrap();

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

// YTS PAGE cant be <= 1
//  1 < page_size <= 50
// title must not contain white space, replace space with '-'

pub async fn query_yts_content_provider(
    metadata: &SearchQueryMetadata,
) -> Result<MovieList, Box<dyn std::error::Error + Send + Sync>> {
    println!("metaData : {:#?}", metadata);

    let mut yts_movie_client = ListMovies::new();
    let mut res = yts_movie_client
        .limit(metadata.page_size)
        .query_term(&metadata.query_term)
        .wirth_rt_ratings(metadata.with_rt_ratings);
    let mut genre_str = String::new();
    if metadata.genre.is_some() {
        let genre = metadata.genre.clone().unwrap();
        genre_str = genre.to_string();
    };
    if metadata.page > 1 {
        res = res.page(metadata.page)
    }
    if metadata.quality.is_some() {
        let q = match metadata.quality.unwrap() {
            MovieQuality::Q1080p => Quality::Q1080p,
            MovieQuality::Q720p => Quality::Q720p,
            MovieQuality::Q2160p => Quality::Q2160p,
            MovieQuality::Q3D => Quality::Q3D,
        };
        res = res.quality(q);
    }
    if metadata.sort_by.is_some() {
        let s = match metadata.sort_by.clone().unwrap() {
            SortBy::DownloadCount => Sort::DownloadCount,
            SortBy::Title => Sort::Title,
            SortBy::Year => Sort::Year,
            SortBy::Rating => Sort::Rating,
            SortBy::Peers => Sort::Peers,
            SortBy::Seeds => Sort::Seeds,
            SortBy::LikeCount => Sort::LikeCount,
            SortBy::DateAdded => Sort::DateAdded,
        };
        res = res.sort_by(s);
    }
    if metadata.order_by.is_some() {
        let o = match metadata.order_by.clone().unwrap() {
            SearchOrder::Asc => Order::Asc,
            SearchOrder::Desc => Order::Desc,
        };
        res = res.order_by(o);
    }
    if !genre_str.is_empty() {
        res = res.genre(genre_str.as_str());
    }

    let res = res.execute().await?;
    Ok(res)
}


pub async fn yts_movie_search_handler(connection: &Data<PgPool>, query_span: Span, search_params: &SearchQueryMetadata) -> Result<serde_json::Value, String> {
    let yts_res = query_yts_content_provider(search_params).await;
        let movie_list = match yts_res {
            Ok(list) => list,
            Err(err) => {
                tracing::error!("YTS HANDLER ERR");
                    return Err(err.to_string());
            }
        };

        if movie_list.movies.is_empty() {
            return Ok(json!({
                "limit": search_params.page_size,
                "max_movie_count": 0,
                "max_page_count": 0
            }));
        }

        let watched_movies_ids_res =
            get_watched_movies_ids(&movie_list, &connection, query_span.clone()).await;

        let response = match watched_movies_ids_res {
            Ok(ids) => {
                tracing::info!("Got Watched Movies ids");
                format_movie_list_response(&ids, &movie_list)
            }
            Err(err) => {
                tracing::error!("Database Error");
                return Err(err.to_string());
            }
        };
        Ok(response)
}

pub async fn movie_db_handler(_connection: &Data<PgPool>, _query_span: Span, _search_params: &SearchQueryMetadata) -> Result<serde_json::Value, String> {
    todo!()
}
