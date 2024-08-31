use actix_web::HttpMessage;
use actix_web::{
    http,
    web::{Data, Path, Query},
    HttpRequest, HttpResponse,
};
use serde_json::json;
use sqlx::PgPool;
use std::rc::Rc;
use tracing::Instrument;

use crate::middleware::User;

async fn get_watched_movies(connection: Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let span = tracing::info_span!("Getting watched movies list");
    let user_id = {
        let extension = req.extensions();
        let user_option: Option<&Rc<User>> = extension.get::<Rc<User>>();
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

    // match sqlx::query!(r#"
    //   SELECT * FROM watched_movies WHERE user_id = $1
    // "#,
    // user_uid
    // ).fetch_all(connection.as_ref())
    // .instrument(span).await {
    //   Ok(res) => {

    //   },
    //   Err(err) =>{
    //     tracing::error!("Data Error {}", err);
    //     return HttpResponse::BadRequest().json(json!({
    //       "Error": "Something went wrong"
    //     }))
    //   }
    // }

    todo!()
}
