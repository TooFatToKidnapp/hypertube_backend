use std::rc::Rc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use actix_web::{web::{Data, Json}, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::middleware::User;

#[derive(Deserialize)]
pub struct FavoriteMovie {
    movie_id: String,
    title : String,
    movie_imdb_code: Option<String>,
    movie_source: String,
    poster_src: String,
    rating: String,
    genres: Vec<String>,
}

pub async fn set_favorite_movie(
    connection: Data<PgPool>,
    req: HttpRequest,
    movie: Json<FavoriteMovie>,
) -> HttpResponse {
    tracing::info!("SET FAVORITE MOVIE");

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
        "INSERT INTO favorite_movies (user_id, movie_id, title ,movie_imdb_code, movie_source, poster_src, rating, genres, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        visitor_id,
        movie.movie_id,
        movie.title,
        movie.movie_imdb_code,
        movie.movie_source,
        movie.poster_src,
        movie.rating,
        &movie.genres,
        created_at
    );

    match query.execute(&**connection).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Favorite movie added successfully"
        })),
        Err(e) => {
            tracing::error!("Database query failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to add favorite movie"
            }))
        }
    }
}