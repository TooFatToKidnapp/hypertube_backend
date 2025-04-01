use actix_web::web::Data;
use actix_web::{web, Scope};
use crate::middleware::Authentication;
use sqlx::{PgPool, Row};
use crate::routes::subtitles::search_subtitles::get_subtiles_search;
use crate::routes::subtitles::download_subtitle::download_subtile_file;
use crate::routes::subtitles::search_subtitles::RequestParam;
// use super::{
//     get_subtiles_search,
// };

pub fn subtitle_source(db_pool: &PgPool) -> Scope {
    web::scope("/subtitles")
        .route(
            "/search/{imdb_id}",
            web::get()
                .to(get_subtiles_search)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/download/{file_id}",
            web::get()
                .to(download_subtile_file)
                .wrap(Authentication::new(db_pool.clone())),
        )
}