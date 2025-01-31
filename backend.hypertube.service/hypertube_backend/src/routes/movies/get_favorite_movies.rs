use std::rc::Rc;

use serde_json::json;
use sqlx::PgPool;
use actix_web::{http, web::{Data, Path}, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::middleware::User;



// #[derive(Clone, Debug)]
// pub struct User {
//     pub id: Uuid,
//     pub first_name: Option<String>,
//     pub last_name: Option<String>,
//     pub image_url: Option<String>,
//     pub username: String,
//     pub email: String,
//     pub created_at: String,
//     pub updated_at: String,
//     pub session_id: Option<Uuid>,
// }



pub async fn get_favorite_movies(
    connection: Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    tracing::info!("FAVORITE MOVIES");

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
        // "SELECT movie_id, movie_imdb_code, title, poster_src, movie_source, created_at FROM favorite_movies WHERE user_id = $1",
        "SELECT * FROM favorite_movies WHERE user_id = $1",
        visitor_id
    );

    match query.fetch_all(connection.get_ref()).await {
        Ok(favorite_movies) => {
            let movies: Vec<_> = favorite_movies.iter().map(|record| {
                json!({
                    "movie_id": record.movie_id,
                    "movie_imdb_code": record.movie_imdb_code,
                    "movie_source": record.movie_source,
                    "created_at": record.created_at,
                    "rating": record.rating,
                    "genres": record.genres,
                    "poster_src": record.poster_src,
                    "title":record.title,
                })
            }).collect();

            HttpResponse::Ok().json(movies)
        }
        Err(e) => {
            tracing::error!("Database query failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch favorite movies"
            }))
        }
    }
}
