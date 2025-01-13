use std::rc::Rc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use actix_web::{web::{Data, Json}, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::middleware::User;

#[derive(Deserialize)]
pub struct WatchedMovie {
    movie_id: String,
    movie_imdb_code: Option<String>,
    movie_source: String,
}

pub async fn set_watched_movie(
    connection: Data<PgPool>,
    req: HttpRequest,
    movie: Json<WatchedMovie>,
) -> HttpResponse {
    tracing::info!("SET WATCHED MOVIE");

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

    let created_at: DateTime<Utc> = Utc::now();

    let query = sqlx::query!(
        "INSERT INTO watched_movies (user_id, movie_id, movie_imdb_code, movie_source, created_at) VALUES ($1, $2, $3, $4, $5)",
        visitor_id,
        movie.movie_id,
        movie.movie_imdb_code,
        movie.movie_source,
        created_at
    );

    match query.execute(&**connection).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Watched movie added successfully"
        })),
        Err(e) => {
            tracing::error!("Database query failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to add watched movie"
            }))
        }
    }
}