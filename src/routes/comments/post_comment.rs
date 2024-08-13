use std::rc::Rc;

use actix_web::{
    web::{Data, Json},
    HttpMessage, HttpRequest, HttpResponse,
};
use tracing::Instrument;

use super::validate_user_comment;
use crate::{middleware::User, routes::Source};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate, Debug)]
pub struct CommentPayload {
    #[validate(custom(function = "validate_user_comment"))]
    pub comment: String,
    pub movie_id: i32,
    pub source: Source,
}

pub async fn post_comment(
    connection: Data<PgPool>,
    body: Json<CommentPayload>,
    req: HttpRequest,
) -> HttpResponse {
    let query_span = tracing::info_span!("Save user Comment", ?body);

    let is_valid = body.validate();
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

    let user_id = {
        let user_info = req.extensions();
        let user_option = user_info.get::<Rc<User>>();
        match user_option {
            Some(user) => user.id,
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "No user info in request payload"
                }));
            }
        }
    };

    match sqlx::query(
        r#"
      INSERT INTO user_comments (id ,movie_source, movie_id, created_at, user_id, comment)
      VALUES ($1, $2, $3, $4, $5, $6)
    "#,
    )
    .bind(Uuid::new_v4())
    .bind(body.source.clone() as Source)
    .bind(body.movie_id)
    .bind(Utc::now())
    .bind(user_id)
    .bind(body.comment.clone())
    .execute(connection.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            HttpResponse::BadRequest().json(json!({
              "error": "Something went wrong"
            }))
        }
    }
}
