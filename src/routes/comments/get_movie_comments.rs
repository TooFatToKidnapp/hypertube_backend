use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::routes::Source;

#[derive(Deserialize, Debug)]
pub struct MovieIdentifier {
    pub movie_id: i32,
    pub source: Source,
}

#[derive(Deserialize, Debug)]
pub struct PageInfo {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

pub async fn get_movie_comments(
    connection: Data<PgPool>,
    movie_info: Path<MovieIdentifier>,
    page_info: Query<PageInfo>,
) -> HttpResponse {
    let movie_info = movie_info.into_inner();
    let query_span = tracing::info_span!("Get movie comments");

    let page = page_info.page.unwrap_or(1);
    let page_size = page_info.page_size.unwrap_or(10);

    // match
    todo!()
}
