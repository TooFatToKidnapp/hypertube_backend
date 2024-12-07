use crate::{middleware::User, routes::validate_uuid};
use actix_web::{
    web::{Data, Path},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::rc::Rc;
use tracing::Instrument;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CommentId {
    #[validate(custom(function = "validate_uuid"))]
    pub comment_id: String,
}

pub async fn delete_comment(
    connection: Data<PgPool>,
    path: Path<CommentId>,
    req: HttpRequest,
) -> HttpResponse {
    let query_span = tracing::info_span!("Delete user comment");

    let is_valid = path.validate();
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
    let comment_id = match path.into_inner().comment_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(err) => {
            tracing::error!("Invalid comment uuid {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
              "error": "Bad Comment uuid"
            }));
        }
    };

    let user_id = {
        let extension = req.extensions();
        let user_option = extension.get::<Rc<User>>();
        match user_option {
            Some(user) => user.id,
            None => {
                tracing::info!("User field not found in req object");
                return HttpResponse::NotFound().json(json!({
                    "error": "user not found"
                }));
            }
        }
    };
    match sqlx::query(
        r#"
      DELETE FROM user_comments WHERE id = $1 AND user_id = $2
    "#,
    )
    .bind(comment_id)
    .bind(user_id)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(res) => {
            if res.rows_affected() > 0 {
                tracing::info!("Comment delete successfully");
                HttpResponse::Ok().finish()
            } else {
                tracing::info!("Comment or user not found in the database");
                HttpResponse::NotFound().finish()
            }
        }
        Err(err) => {
            tracing::error!("Database error {:#?}", err);
            HttpResponse::BadRequest().json(json!({
              "error" : "Something went wrong"
            }))
        }
    }
}
