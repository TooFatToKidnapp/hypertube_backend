use super::validate_title;
use actix_web::{
    web::{Data, Json, Query},
    HttpResponse,
};
use std::fmt;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;
use yts_api::{ListMovies, MovieList};
// https://ww4.yts.nz/api
// https://popcornofficial.docs.apiary.io/#reference/show/get-page/pages
// https://crates.io/crates/yts-api
// https://developer.themoviedb.org/docs/getting-started

#[derive(Deserialize, Debug)]
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
        write!(f, "{}",
        match self {
            Genre::Action => "Action",
            Genre::Drama => "Drama",
            Genre::Thriller => "Thriller",
            Genre::Crime => "Crime",
            Genre::Adventure => "Adventure",
            Genre::Comedy => "Comedy",
            Genre::SciFi => "SciFi",
            Genre::Romance => "Romance",
            Genre::Fantasy => "Fantasy",
            Genre::Horror => "Horror",
            Genre::Mystery => "Mystery",
            Genre::War => "War",
            Genre::Animation => "Animation",
            Genre::History => "History",
            Genre::Family => "Family",
            Genre::Western => "Western",
            Genre::Sport => "Sport",
            Genre::Documentary => "Documentary",
            Genre::Biography => "Biography",
            Genre::Musical => "Musical",
            Genre::Music => "Music",
            Genre::FilmNoir => "FilmNoir",
            Genre::News => "News",
            Genre::RealityTV => "RealityTV",
            Genre::GameShow => "GameShow",
            Genre::TalkShow => "TalkShow",
        })
    }
}

#[derive(Deserialize, Debug, Default, PartialEq, Clone)]
pub enum Source {
    #[default]
    YTS,
    PopcornOfficial,
    MovieDb,
}

#[derive(Deserialize, Validate, Debug)]
pub struct SearchBody {
    #[validate(custom(function = "validate_title"))]
    pub title: String,
    pub genre: Option<Genre>,
    pub source: Option<Source>,
    // pub date: Option<>
}

#[derive(Deserialize)]
pub struct Paginate {
    pub page_size: Option<u8>,
    pub page: Option<u32>,
}

pub enum SearchOrder {
    Desc,
    Asc,
}

struct SearchQueryMetadata {
    pub page: u32,
    pub page_size: u8,
    pub source: Source,
    pub quality: String,
    pub minimum_rating: u8,
    pub query_term: String,
    pub genre: Genre,
    pub sort_by: String,
    pub order_by: SearchOrder,
    with_rt_ratings: bool,
}

// YTS PAGE cant be <= 1
//  1 < page_size <= 50
// title must not contain white space, replace space with '-'

async fn query_yts_content_provider(
    page: u32,
    page_size: u8,
    search_query: &str,
) -> Result<MovieList, Box<dyn std::error::Error + Send + Sync>> {
    let mut yts_movie_client = ListMovies::new();
    let mut res = yts_movie_client.limit(page_size).query_term(search_query);
    if page > 1 {
        res = res.page(page)
    }
    let res = res.execute().await?;
    Ok(res)
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

    let page = info.page.unwrap_or(0u32);
    let page_size = info.page_size.unwrap_or(10u8);
    let content_provider = body.source.clone().unwrap_or_default();

    if content_provider == Source::YTS {
        tracing::info!("Calling the YTS Handler");
        let yts_res = query_yts_content_provider(page, page_size, &body.title).await;
        match yts_res {
            Ok(list) => return HttpResponse::Ok().json(list),
            Err(err) => {
                tracing::error!("YTS HANDLER ERR {:#?}", err);
                return HttpResponse::InternalServerError().json(json!({
                    "error": err.to_string()
                }));
            }
        };
    }
    HttpResponse::Ok().finish()
}
