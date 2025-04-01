use std::rc::Rc;
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use actix_web::{web::{Data, Path}, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;
use crate::middleware::User;
use serde::Deserialize;
use validator::Validate;
use crate::routes::user::validate_uuid;

#[derive(Serialize)]
pub struct WatchedMovie {
    movie_id: String,
    title : String,
    movie_imdb_code: Option<String>,
    movie_source: String,
    poster_src: String,
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
        "SELECT movie_id, movie_imdb_code, title, poster_src, movie_source, created_at FROM watched_movies WHERE user_id = $1",
        visitor_id
    );

    // let query = sqlx::query!(
    //     "SELECT movie_id, movie_imdb_code, movie_source, created_at FROM watched_movies WHERE user_id = $1",
    //     visitor_id
    // );

    match query.fetch_all(&**connection).await {
        Ok(rows) => {
            let watched_movies: Vec<WatchedMovie> = rows.into_iter().map(|row| WatchedMovie {
                movie_id: row.movie_id,
                movie_imdb_code: row.movie_imdb_code,
                title: row.title,
                poster_src: row.poster_src,
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

#[derive(Deserialize, Validate)]
pub struct RequestParam {
    #[validate(custom(function = "validate_uuid"))]
    pub id: String,
}

pub async fn get_user_watched_movies(
    connection: Data<PgPool>,
    path: Path<RequestParam>,
) -> HttpResponse {
    tracing::info!("GET WATCHED MOVIES");

    let parsed_user_id = match path.id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(err) => {
            tracing::error!("Error parsing param id {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Error parsing param id"
            }));
        }
    };

    let query = sqlx::query!(
        "SELECT movie_id, movie_imdb_code, title, poster_src, movie_source, created_at FROM watched_movies WHERE user_id = $1",
        parsed_user_id
    );

    match query.fetch_all(connection.get_ref()).await {
        Ok(rows) => {
            let watched_movies: Vec<WatchedMovie> = rows.into_iter().map(|row| WatchedMovie {
                movie_id: row.movie_id,
                movie_imdb_code: row.movie_imdb_code,
                title: row.title,
                poster_src: row.poster_src,
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