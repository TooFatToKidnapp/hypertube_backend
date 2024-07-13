use actix_web::{web::{Data, Json}, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use validator::Validate;

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

#[derive(Deserialize, Debug)]
pub enum Source {
    YTS,
    PopcornOfficial,
    MovieDb
}
use super::{validate_title};

#[derive(Deserialize, Validate, Debug)]
pub struct SearchBody {
    #[validate(custom(function = "validate_title"))]
    pub title: Option<String>,
    pub genre: Option<Genre>,
    pub source: Option<Source>

}

pub async fn get_movie_list(connection: Data<PgPool>, body: Json<SearchBody>) -> HttpResponse {
    println!("{:#?}", body);

    HttpResponse::Ok().finish()
}
