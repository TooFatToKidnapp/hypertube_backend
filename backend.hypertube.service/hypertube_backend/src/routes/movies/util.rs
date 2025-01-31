use super::{
    delete_torrent, get_favorite_movies, get_movie_info, get_movie_subtitles, get_movies_search, get_watched_movies, get_yts_top_movies, get_yts_top_movies_in_genre, remove_favorite_movie, set_favorite_movie, set_watched_movie, stream_video_content
};
use crate::middleware::Authentication;
use crate::routes::download_torrent;
use actix_web::web::Data;
use actix_web::{http, web, Scope};
use chrono::NaiveDate;
use lettre::transport::smtp::response;
use serde::Deserialize;
use serde_json::{json, Number, Value};
use sqlx::{PgPool, Row};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::{fmt, result};
use tracing::{Instrument, Span};
use validator::ValidationError;
use yts_api::{ListMovies, MovieList, Order, Quality, Sort};
use crate::routes::movies::types::ImdbMovieDetails;
use bigdecimal::BigDecimal;
use std::str::FromStr;

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
        .route("/favorite",
            web::get()
            .to(get_favorite_movies)
            .wrap(Authentication::new(db_pool.clone())),
        )
        .route("/favorite",
            web::post()
            .to(set_favorite_movie)
            .wrap(Authentication::new(db_pool.clone())),
        )
        .route("/favorite",
            web::delete()
            .to(remove_favorite_movie)
            .wrap(Authentication::new(db_pool.clone())),
        )
        .route("/history",
        web::get()
            .to(get_watched_movies)
            .wrap(Authentication::new(db_pool.clone())),
        )
        .route("/history",
        web::post()
            .to(set_watched_movie)
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
        // id shouldn't be expected to be a number it containes tt at the start
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

async fn find_torrents(search_params: &SearchQueryMetadata) -> Result<Vec<Value>, String> {

    let client = reqwest::Client::new();

    let imdb_search_torrent_host = std::env::var("IMDB_SEARCH_TORRENT_HOST").unwrap();
    // let imdb_search_torrrent_endpoint = std::env::var("IMDB_SEARCH_TORRENT_ENDPOINT").unwrap();
    let imdb_search_torrent_token = std::env::var("IMDB_SEARCH_TORRENT_TOKEN").unwrap();

    let search_url = format!(
        "https://{}/search/{}",
        imdb_search_torrent_host,
        search_params.query_term,
        // search_params.page
    );
    
    let search_torrent_res = client
        .get(search_url)
        .header(
            "x-rapidapi-key",
            &imdb_search_torrent_token,
        ).header("x-rapidapi-host", &imdb_search_torrent_host)
        .send()
        .await;

    let response = match search_torrent_res {
        Ok(res) => {
            tracing::info!("RESPONSE ERROR ::: {:#?}", &res);
            tracing::info!("Got Movie db search response");
            res
        }
        Err(err) => {
            tracing::error!("MOVIE DB request error {:#?}", err);
            return Err(err.to_string());
        }
    };

    if response.status() == 429{
        return Err(String::from("you exceeded your daily QUOTA"));
    }

    let response_body = response.json::<serde_json::Value>().await;
    let res = match response_body {
        Ok(val) => {val}
        Err(res_err) =>{
            tracing::error!("Parsing response body error {:#?}", res_err);
            return Err("Failed to parse response body".to_string());
        }
    };

    if res["data"].as_array().is_none(){
        return  Err(String::from("no movie data was provided"));
    }

    let movie_torrent_arr = res["data"].as_array().unwrap();

    return Ok(movie_torrent_arr.clone());
}


async fn get_top_imdb_movies() -> Result<Vec<Value>, String> {

    let client = reqwest::Client::new();

    let imdb_search_host = std::env::var("IMDB_SEARCH_HOST").unwrap();
    let imdb_search_token = std::env::var("IMDB_SEARCH_TOKEN").unwrap();

    let search_url = format!("https://{}/imdb/top250-movies",
    imdb_search_host
    );

    let search_movie_res = client
    .get(search_url)
    .header("x-rapidapi-key", &imdb_search_token)
    .header("x-rapidapi-host",& imdb_search_host)
    .send()
    .await;

    let response = match search_movie_res {
    Ok(val) => { 
        tracing::info!("Got IMDB search response");
        tracing::info!("IMDB search response:: {:#?}", val);
        val 
    }
    Err(err) => {
        tracing::error!("SEARCH MOVIE ERROR : {:#?}", err);
        return Err(err.to_string());
    }
    };

    if response.status() == 429{
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

    if res["items"].as_array().is_none() {
    tracing::error!("No result in search response body");
    return Err(String::from("No result in search response body"));
    }

    let movie_search_arr = res["items"].as_array().unwrap();

    return Ok(movie_search_arr.clone());
}





async fn find_movies(search_params: &SearchQueryMetadata) -> Result<Vec<Value>, String> {

    let client = reqwest::Client::new();

    let imdb_search_host = std::env::var("IMDB_SEARCH_HOST").unwrap();
    let imdb_search_token = std::env::var("IMDB_SEARCH_TOKEN").unwrap();

    let search_url = format!("https://{}/imdb/search?originalTitle={}&type=movie&sortField=id&sortOrder=ASC",
    imdb_search_host,
    search_params.query_term);

    let search_movie_res = client
    .get(search_url)
    .header("x-rapidapi-key", &imdb_search_token)
    .header("x-rapidapi-host",& imdb_search_host)
    .send()
    .await;

    let response = match search_movie_res {
    Ok(val) => { 
        tracing::info!("Got IMDB search response");
        tracing::info!("IMDB search response:: {:#?}", val);
        val 
    }
    Err(err) => {
        tracing::error!("SEARCH MOVIE ERROR : {:#?}", err);
        return Err(err.to_string());
    }
    };

    if response.status() == 429{
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

    if res["results"].as_array().is_none() {
    tracing::error!("No result in search response body");
    return Err(String::from("No result in search response body"));
    }

    let movie_search_arr = res["results"].as_array().unwrap();

    return Ok(movie_search_arr.clone());
}

async fn save_imdb_movie_details(
    connection: &Data<PgPool>,
    query_span: Span,
    movie: ImdbMovieDetails,
) {

        let query_res = sqlx::query!(
            r#"
            INSERT INTO imdb_movie_details (
                id, primary_title, original_title, source_type, genres, is_adult, start_year, end_year,
                runtime_minutes, average_rating, num_votes, description, primary_image, content_rating,
                release_date, interests, countries_of_origin, external_links, spoken_languages,
                filming_locations, budget, gross_world_wide, directors, writers, "cast", torrents
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26
            )
            Returning *
            "#,
            movie.id,
            movie.primary_title,
            movie.original_title,
            movie.source_type,
            &movie.genres,
            movie.is_adult,
            movie.start_year,
            movie.end_year,
            movie.runtime_minutes,
            movie.average_rating,
            movie.num_votes,
            movie.description,
            movie.primary_image,
            movie.content_rating,
            movie.release_date,
            &movie.interests,
            &movie.countries_of_origin,
            &movie.external_links,
            &movie.spoken_languages,
            &movie.filming_locations,
            movie.budget,
            movie.gross_world_wide,
            &movie.directors,
            &movie.writers,
            &movie.cast,
            &movie.torrents
        )
        .fetch_one(connection.get_ref())
        .instrument(query_span.clone())
        .await;

        let res = match query_res {
            Ok(_val) => {
                tracing::error!("GOT QUERY ");
                // dbg!("Got query:: {:#?}", val);
            }
            Err(err) => {
                tracing::error!("QUERY FAILED: {:#?}", err);
            }
        };

}


async fn get_movies_list(
    connection: &Data<PgPool>,
    query_span: Span,
    movie_search_arr: &Vec<Value>,
    movie_torrent_arr :&Vec<Value> ) -> Result<HashMap<String, ImdbMovieDetails>, String> {

    let client = reqwest::Client::new();

    let imdb_search_host = std::env::var("IMDB_SEARCH_HOST").unwrap();
    let imdb_search_token = std::env::var("IMDB_SEARCH_TOKEN").unwrap();

    let mut movies_db :HashMap<String, ImdbMovieDetails> = HashMap::new();
    // let mut movies_list :Vec<Value> = Vec::new();

    let mut filtered_movies: HashMap<&str, Value> = HashMap::new();
    for movie in movie_search_arr {
        for torrent in movie_torrent_arr {
            let imdb_id = torrent["imdb"].as_str();
            if imdb_id.is_none() {
                continue;
            }
            if movie["id"].as_str().is_none(){
                continue;
            }

            let imdb_id = imdb_id.unwrap();
            if movie["id"].as_str().unwrap() == imdb_id{
                if filtered_movies.contains_key(imdb_id){
                    continue;
                }
                filtered_movies.insert(imdb_id, movie.clone());
            }
        }
    }

    for (id, _movie) in filtered_movies {

        // let movie_id = movie["id"].as_str().unwrap();
        let movie: Value;

        let search_url = format!("https://{}/imdb/{}", 
                                &imdb_search_host,
                                id
                                // movie_id
                                );

        let response = client.get(search_url)
                                    .header("x-rapidapi-host", &imdb_search_host)
                                    .header("x-rapidapi-key", &imdb_search_token)
                                    .send()
                                    .await;

        match response {
            Ok(val) => {

                if val.status() == 429{
                    return Err(String::from("you exceeded your daily QUOTA"));
                }

                let response_body = val.json::<serde_json::Value>().await;
                let response_body = match response_body {
                    Ok(val) => {Some(val)}
                    Err(err) => {tracing::info!("Couldn't get body for movie id : [{}]", id); None}
                };

                if response_body.is_none() {
                    continue;
                }

                let response_body = response_body.unwrap();
                
                movie = response_body;
                // movies_list.push(movie.clone());

            }

            Err(err) => {
                tracing::error!("Couldn't get response body, [ {} ]", err);
                return Err(err.to_string());
            }
        };


        for torrent in movie_torrent_arr {
            let imdb_id = torrent["imdb"].as_str();
            if imdb_id.is_none() {
                continue;
            }
            // if movie["id"].as_str().is_none(){
            //     continue;
            // }

            let imdb_id = imdb_id.unwrap();
            if id == imdb_id {
                tracing::info!("Got a match: {}", &imdb_id);
                if movies_db.contains_key(imdb_id) {
                    tracing::info!("A key {}", imdb_id);
                    let details = movies_db.get_mut(imdb_id);
                    if details.is_none(){
                        continue;
                    }
                    let details = details.unwrap();
                    details.torrents.push(torrent.clone());
                }
                else {
                    tracing::info!("creating a new pair wit key {}", imdb_id);

                    let mut details = ImdbMovieDetails{
                        id: movie["id"].as_str().unwrap().to_string(),
                        primary_title: movie["primaryTitle"].as_str().unwrap().to_string(),
                        original_title: movie["originalTitle"].as_str().unwrap().to_string(),
                        source_type: movie["type"].as_str().unwrap().to_string(),
                        genres: movie["genres"].as_array().unwrap().iter().map(|g| g.as_str().unwrap().to_string()).collect(),
                        is_adult: movie["isAdult"].as_bool().unwrap(),
                        start_year: movie["startYear"].as_i64().unwrap() as i32,
                        end_year: 0,
                        runtime_minutes: movie["runtimeMinutes"].as_i64().unwrap() as i32,
                        average_rating: movie["averageRating"].as_f64().unwrap().to_string(),
                        num_votes: movie["numVotes"].as_i64().unwrap_or(-1) as i32,
                        description: movie["description"].as_str().unwrap().to_string(),
                        primary_image: movie["primaryImage"].as_str().unwrap().to_string(),
                        content_rating: movie["contentRating"].as_str().unwrap().to_string(),
                        release_date: None::<NaiveDate>,//String::from("to be set"),
                        interests: movie["interests"].as_array().unwrap().iter().map(|i| i.as_str().unwrap().to_string()).collect(),
                        countries_of_origin: movie["interests"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect(),
                        external_links: movie["interests"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect(),
                        spoken_languages: movie["interests"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect(),
                        filming_locations: movie["interests"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect(),
                        directors: movie["directors"].as_array().unwrap().clone(),
                        writers: movie["directors"].as_array().unwrap().clone(),
                        cast: movie["directors"].as_array().unwrap().clone(),
                        gross_world_wide: movie["budget"].as_i64().unwrap_or(-1) as i32,
                        budget: movie["budget"].as_i64().unwrap_or(-1) as i32,
                        torrents: Vec::new(),
                    };
                    details.torrents.push(torrent.clone());
                    movies_db.insert(String::from(imdb_id), details.clone());
                }
            }
        }

    };

    tracing::info!("MOVIE DB---------------------------");
    for (key, value) in &movies_db {
        tracing::info!("K: {}", key);
        tracing::info!("V: {:#?}", value);
        // println!("Value: {:?}", value);
    }
    tracing::info!("end MOVIE DB---------------------------");

    for (_, movie) in &movies_db {
        // movie_set.insert(movie.clone());
        save_imdb_movie_details(connection, query_span.clone(), movie.clone()).await;
    }
    return Ok(movies_db);
}


pub async fn movie_db_handler(
    connection: &Data<PgPool>,
    query_span: Span,
    search_params: &SearchQueryMetadata,
) -> Result<serde_json::Value, String> {

    ////////// SEARCH TORRENT ////////////////
    
    let find_torrent_res = find_torrents(search_params).await;
    let movie_torrent_arr = match find_torrent_res {
        Ok(val) => val,
        Err(err) => {return Err(err);}
    };

    tracing::info!("MOVIE torrents :: {:#?}", movie_torrent_arr);

    ////////// SEARCH MOVIES /////////////////

    let find_movies_res = find_movies(search_params).await;

    let movie_search_arr = match find_movies_res {
        Ok(val) => {
            val
        }
        Err(err) => {
            return Err(err);
        }
    };

    ////////// SEARCH MOVIES END /////////////

    let movies_list_res = get_movies_list(connection, query_span.clone(), &movie_search_arr, &movie_torrent_arr).await;

    let movies_list = match movies_list_res {
        Ok(val) => {
            val
        }
        Err(err) => {
            return Err(err);
        }
    };

    let mut client_response = json!({
        "limit": 0,
    //     "max_movie_count": res["total_results"].as_i64().unwrap_or(0),
    //     "max_page_count": res["total_pages"].as_i64().unwrap_or(0),
        "movies": []
    });

    if movies_list.is_empty() {
        return Ok(client_response);
    }

    //////////////////////////////////////////
    // need to add the watched movies logic///
    //////////////////////////////////////////

    // let watched_movies_ids = match get_movie_db_watched_ids(&movies_list, connection, query_span).await
    // {
    //     Ok(ids) => {
    //         tracing::info!("Got watched movies ids");
    //         ids
    //     }
    //     Err(err) => {
    //         tracing::info!("Movie db database error {:#?}", err);
    //         return Err(err.to_string());
    //     }
    // };
    // println!("watched movies arr {:#?}", watched_movies_ids);

    let client_res_movie_arr = client_response["movies"].as_array_mut().unwrap();

    for (id, movie) in movies_list{

        let mut movie_content = json!({
            "genres": movie.genres,
            "id": movie.id,
            "language": movie.spoken_languages,
            "large_cover_image": movie.primary_image,
            "large_screenshot_image1": movie.primary_image,
            "year": movie.start_year,
            "runtime": movie.runtime_minutes,
            // "rating":movie.content_rating,
            "small_cover_image": None::<String>,
            "description_intro": movie.description ,
            "synopsis": movie.description,
            "title": movie.original_title,
            "title_english": movie.primary_title,
            "title_long": None::<String>,
            "rating": movie.average_rating,
            "watched": false
        });
        // if watched_movies_ids.contains(&movie_content["id"].as_number().unwrap().to_string()) {
        //     movie_content["watched"] = json!(true);
        // }
        client_res_movie_arr.push(movie_content);
    }

    let paginated_movies = paginate_movies(client_res_movie_arr, search_params.page, search_params.page_size);
    client_response["movies"] = json!(paginated_movies);
    client_response["limit"] = json!(paginated_movies.len());

    Ok(client_response)
}

fn paginate_movies(client_res_movie_arr: &mut Vec<Value>, page: u32, page_size: u8) -> Vec<Value> {
    let page = page as usize;
    let page_size = page_size as usize;
    if page == 0 || page_size == 0 {
        return vec![];
    }

    let start = (page - 1) * page_size;
    if start >= client_res_movie_arr.len() {
        return vec![];
    }

    let end = start + page_size;
    client_res_movie_arr[start..end.min(client_res_movie_arr.len())].to_vec()
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
