use std::rc::Rc;
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use actix_web::{web::Data, HttpMessage, HttpRequest, HttpResponse};
use crate::middleware::User;

#[derive(Serialize)]
pub struct WatchedMovie {
    movie_id: String,
    movie_imdb_code: Option<String>,
    movie_source: String,
    created_at: String,
}

pub async fn get_watched_movies(
    connection: Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    tracing::info!("GET WATCHED MOVIES");

    let visitor_id = {
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

    let query = sqlx::query!(
        "SELECT movie_id, movie_imdb_code, movie_source, created_at FROM watched_movies WHERE user_id = $1",
        visitor_id
    );

    match query.fetch_all(&**connection).await {
        Ok(rows) => {
            let watched_movies: Vec<WatchedMovie> = rows.into_iter().map(|row| WatchedMovie {
                movie_id: row.movie_id,
                movie_imdb_code: row.movie_imdb_code,
                movie_source: row.movie_source,
                created_at: row.created_at.to_string(),
            }).collect();
            HttpResponse::Ok().json(watched_movies)
        },
        Err(e) => {
            tracing::error!("Database query failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch watched movies"
            }))
        }
    }
}