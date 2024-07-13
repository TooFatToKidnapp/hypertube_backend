use std::borrow::Cow;

use actix_web::{
    web::{self, get, patch, post},
    Scope,
};
use sqlx::PgPool;
use validator::ValidationError;

use crate::middleware::Authentication;

use super::{get_movie_list, Source};

pub fn movie_source(db_pool: &PgPool) -> Scope {
    web::scope("/movies").route(
        "/search",
        web::post()
            .to(get_movie_list)
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

