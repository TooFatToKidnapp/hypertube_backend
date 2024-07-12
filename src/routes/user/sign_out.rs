use crate::middleware::User;
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::{cookie::Cookie, web::Data, HttpMessage, HttpRequest, HttpResponse};
use serde_json::json;
use sqlx::PgPool;
use std::rc::Rc;
use tracing::Instrument;

pub async fn sign_out_user(connection: Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let query_span = tracing::info_span!("User sign-out event");

    let session_id = {
        let extension = req.extensions();
        let id_option = match extension.get::<Rc<User>>() {
            Some(user) => user.session_id,
            None => {
                tracing::error!("User info not found in request payload");
                return HttpResponse::BadRequest().json(json!({
                    "error": "something went wrong"
                }));
            }
        };
        if id_option.is_none() {
            tracing::error!("User session not found in request payload");
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
        id_option.unwrap()
    };
    println!("session id = {session_id:#?}");
    let query_res = sqlx::query(
        r#"
				DELETE FROM sessions WHERE id = $1
		"#,
    )
    .bind(session_id)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await;

    match query_res {
        Ok(_) => {
            tracing::info!("session cleared successfully");
            let cookie = Cookie::build("session", "")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .path("/")
                .max_age(Duration::seconds(0))
                .finish();
            HttpResponse::Ok().cookie(cookie).finish()
        }
        Err(err) => {
            tracing::error!("Database error {:#?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
