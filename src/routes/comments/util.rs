use std::borrow::Cow;

use actix_web::{web, Scope};
use sqlx::PgPool;
use validator::ValidationError;

use crate::middleware::Authentication;

use super::post_comment;

pub fn validate_user_comment(comment: &str) -> Result<(), ValidationError> {
    if comment.is_empty() {
        return Err(ValidationError::new("Invalid Comment")
            .with_message(Cow::from("Comment can't be empty")));
    }

    if comment.trim().is_empty() {
        return Err(ValidationError::new("Invalid Comment")
            .with_message(Cow::from("Comment must contain non white space characters")));
    }

    Ok(())
}

pub fn comment_source(db_pool: &PgPool) -> Scope {
    web::scope("/comments").route(
        "",
        web::post()
            .to(post_comment)
            .wrap(Authentication::new(db_pool.clone())),
    )
}
