use std::rc::Rc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use actix_web::{web::{Data, Json}, HttpMessage, HttpRequest, HttpResponse};
use crate::middleware::User;

#[derive(Deserialize)]
pub struct DeleteFavoriteMovie {
    movie_id: String,
    // movie_source :String,
}

pub async fn remove_favorite_movie(
    connection: Data<PgPool>,
    req: HttpRequest,
    movie: Json<DeleteFavoriteMovie>,
) -> HttpResponse {
    tracing::info!("DELETE FAVORITE MOVIE");

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
        "DELETE FROM favorite_movies WHERE user_id = $1 AND movie_id = $2",
        visitor_id,
        movie.movie_id
    );

    match query.execute(&**connection).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Favorite movie deleted successfully"
        })),
        Err(e) => {
            tracing::error!("Database query failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to delete favorite movie"
            }))
        }
    }
}