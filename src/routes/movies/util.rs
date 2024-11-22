use super::{
    delete_torrent, get_movie_info, get_movie_subtitles, get_movies_search, get_yts_top_movies,
    get_yts_top_movies_in_genre, stream_video_content,
};
use crate::middleware::Authentication;
use crate::routes::download_torrent;
use actix_web::web::Data;
use actix_web::{http, web, Scope};
use serde::Deserialize;
use serde_json::{json, Number, Value};
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
// https://popcorn-official.github.io/popcorn-api/manual/tutorial.html#-trakt-tv-https-trakt-tv-
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
    MovieDb,
}

impl From<String> for Source {
    fn from(value: String) -> Self {
        match value.as_str() {
            "YTS" => Source::YTS,
            "MovieDb" => Source::MovieDb,
            _ => panic!("INVALID CONVERSION"),
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Source::YTS => write!(f, "YTS"),
            Source::MovieDb => write!(f, "MovieDb"),
        }
    }
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

#[derive(Deserialize, Debug, Clone, Default)]
pub enum SortBy {
    #[default]
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
    web::scope("/movies")
        .route("/top", web::get().to(get_yts_top_movies))
        .route("/top/{genre}", web::get().to(get_yts_top_movies_in_genre))
        .route(
            "/search",
            web::post()
                .to(get_movies_search)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/torrent",
            web::post()
                .to(download_torrent)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/delete/{movie_id}/{source}",
            web::delete()
                .to(delete_torrent)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/stream/{source}/{movie_id}/{quality}",
            web::get().to(stream_video_content), // .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/{id}/{source}",
            web::get()
                .to(get_movie_info)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/subtitles",
            web::post()
                .to(get_movie_subtitles)
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

    let data = sqlx::query(query.as_str())
        .fetch_all(connection.as_ref())
        .instrument(span)
        .await?;

    let ids = data
        .iter()
        .map(|row| row.get::<String, &str>("movie_imdb_code"))
        .collect();
    Ok(ids)
}

pub fn format_movie_list_response(ids: &[String], list: &MovieList) -> serde_json::Value {
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
    let search_query = metadata.query_term.to_string().replace(' ', "-");
    println!("metaData : {:#?}", metadata);
    println!("new search query {}", search_query);
    let mut yts_movie_client = ListMovies::new();

    let mut res = yts_movie_client
        .limit(metadata.page_size)
        .query_term(search_query.as_str())
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

pub async fn yts_movie_search_handler(
    connection: &Data<PgPool>,
    query_span: Span,
    search_params: &SearchQueryMetadata,
) -> Result<serde_json::Value, String> {
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
        get_watched_movies_ids(&movie_list, connection, query_span.clone()).await;

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

async fn get_movie_db_watched_ids(
    movie_arr: &[serde_json::Value],
    connection: &Data<PgPool>,
    span: Span,
) -> Result<Vec<String>, sqlx::Error> {
    let movies_imdb_ids: Vec<String> = movie_arr
        .iter()
        .map(|m| format!("'{}'", m["id"].as_number().unwrap_or(&Number::from(0))))
        .collect();
    println!("id list : {:#?}", movies_imdb_ids);
    let query = format!(
        "SELECT * FROM watched_movies WHERE movie_id IN ({})",
        movies_imdb_ids.join(", ")
    );
    let data = sqlx::query(query.as_str())
        .fetch_all(connection.as_ref())
        .instrument(span)
        .await?;

    let ids = data
        .iter()
        .map(|row| row.get::<String, &str>("movie_id"))
        .collect();
    Ok(ids)
}

/*
    Object {
        "adult": Bool(false),
        "backdrop_path": String("/lvoACuXwhmOCr0I29QflIhYGPjd.jpg"),
        "genre_ids": Array [
            Number(16),
            Number(35),
        ],
        "id": Number(160446),
        "original_language": String("ja"),
        "original_title": String("クレヨンしんちゃん 嵐を呼ぶ！夕陽のカスカベボーイズ"),
        "overview": String("At a strange movie theater, everyone gets sucked into the film they're watching. The longer they stay, the less they remember about the real world!"),
        "popularity": Number(14.383),
        "poster_path": String("/aKn6V5ZxAV9REWefExl9LoZLFz8.jpg"),
        "release_date": String("2004-04-16"),
        "title": String("Crayon Shin-chan: Invoke a Storm! The Kasukabe Boys of the Evening Sun"),
        "video": Bool(false),
        "vote_average": Number(7.6),
        "vote_count": Number(16),
    },
*/

// https://www.postman.com/gold-meadow-82853/workspace/tmdb-assignment/collection/5586593-2e0561c2-f870-4b1b-8c17-aaf7cf6c9696
// https://developer.themoviedb.org/reference
// image url: https://image.tmdb.org/t/p/w500/{image path} // https://image.tmdb.org/t/p/original{image_path}
// movie information:  https://api.themoviedb.org/3/find/{imdb id} => get movie id =>  https://api.themoviedb.org/3/movie/{movie id}
// movie search  https://api.themoviedb.org/3/search  {query: {film name}, page: {}}

pub async fn movie_db_handler(
    connection: &Data<PgPool>,
    query_span: Span,
    search_params: &SearchQueryMetadata,
) -> Result<serde_json::Value, String> {
    let search_url = format!(
        "https://api.themoviedb.org/3/search/movie?query={}&include_adult=false&language=en-US&page={}",
        search_params.query_term,
        search_params.page
    );
    let movie_db_token = std::env::var("MOVIE_DB_AUTH_TOKEN").unwrap();
    let client = reqwest::Client::new();
    let search_query_res = client
        .get(search_url)
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", movie_db_token),
        )
        .send()
        .await;

    let response = match search_query_res {
        Ok(res) => {
            tracing::info!("Got Movie db search response");
            res
        }
        Err(err) => {
            tracing::error!("MOVIE DB request error {:#?}", err);
            return Err(err.to_string());
        }
    };

    let res_body_res = response.json::<serde_json::Value>().await;
    if let Err(res_err) = res_body_res {
        tracing::error!("Parsing response body error {:#?}", res_err);
        return Err("Failed to parse response body".to_string());
    }

    let res = res_body_res.unwrap();

    if res["results"].as_array().is_none() {
        return Err("no movie list in response".to_string());
    }
    let movie_arr = res["results"].as_array().unwrap();

    let mut client_response = json!({
        "limit": movie_arr.len(),
        "max_movie_count": res["total_results"].as_i64().unwrap_or(0),
        "max_page_count": res["total_pages"].as_i64().unwrap_or(0),
        "movies": []
    });

    if movie_arr.is_empty() {
        return Ok(client_response);
    }

    let watched_movies_ids = match get_movie_db_watched_ids(movie_arr, connection, query_span).await
    {
        Ok(ids) => {
            tracing::info!("Got watched movies ids");
            ids
        }
        Err(err) => {
            tracing::info!("Movie db database error {:#?}", err);
            return Err(err.to_string());
        }
    };
    println!("watched movies arr {:#?}", watched_movies_ids);

    let client_res_movie_arr = client_response["movies"].as_array_mut().unwrap();

    for movie in movie_arr.iter() {
        if movie["id"].is_null() {
            continue;
        }
        let (large_cover_image, medium_cover_image) = {
            if let Some(url) = movie["poster_path"].as_str() {
                (
                    Some(format!("https://image.tmdb.org/t/p/original{}", url)),
                    Some(format!("https://image.tmdb.org/t/p/w500{}", url)),
                )
            } else {
                (None::<String>, None::<String>)
            }
        };
        let genres = {
            if movie["genre_ids"].as_array().is_some() {
                Some(map_movie_bd_genre_code_with_value(
                    movie["genre_ids"].as_array().unwrap(),
                ))
            } else {
                None::<Vec<Option<&str>>>
            }
        };

        let mut movie_content = json!({
            "genres": genres,
            "id": movie["id"].as_number(),
            "language": movie["original_language"].as_str(),
            "large_cover_image": large_cover_image,
            "medium_cover_image": medium_cover_image,
            "small_cover_image": None::<String>,
            "summary": movie["overview"].as_str(),
            "synopsis": movie["overview"].as_str(),
            "title": movie["original_title"].as_str(),
            "title_english": movie["title"].as_str(),
            "title_long": None::<String>,
            // "rating": movie.rating,
            "watched": false
        });
        if watched_movies_ids.contains(&movie_content["id"].as_number().unwrap().to_string()) {
            movie_content["watched"] = json!(true);
        }
        client_res_movie_arr.push(movie_content);
    }

    Ok(client_response)
}

pub fn map_movie_bd_genre_code_with_value(codes: &[Value]) -> Vec<Option<&str>> {
    let mut genres = Vec::<Option<&str>>::new();

    for code in codes.iter() {
        let code_nb = code.as_i64().unwrap_or(0);
        let genre = match code_nb {
            28 => Some("Action"),
            12 => Some("Adventure"),
            16 => Some("Animation"),
            35 => Some("Comedy"),
            80 => Some("Crime"),
            99 => Some("Documentary"),
            18 => Some("Drama"),
            10751 => Some("Family"),
            14 => Some("Fantasy"),
            36 => Some("History"),
            27 => Some("Horror"),
            10402 => Some("Music"),
            9648 => Some("Mystery"),
            10749 => Some("Romance"),
            878 => Some("Science Fiction"),
            10770 => Some("TV Movie"),
            53 => Some("Thriller"),
            10752 => Some("War"),
            37 => Some("Western"),
            _ => None,
        };
        genres.push(genre);
    }
    genres
}
