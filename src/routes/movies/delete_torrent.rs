use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing::Instrument;
use uuid::Uuid;

use crate::routes::movies::torrent::RqbitWrapper;

use super::Source;

#[derive(Deserialize)]
pub struct MovieData {
    pub movie_id: i32,
    pub source: Source,
}

pub async fn delete_torrent(movie_data: Path<MovieData>, connection: Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Deleting torrent");
    let movie_info = movie_data.into_inner();

    let (torrent_id, record_id, movie_path) = match sqlx::query(
        r#"
      SELECT * FROM movie_torrent WHERE movie_id = $1 AND movie_source = $2
    "#,
    )
    .bind(movie_info.movie_id)
    .bind(movie_info.source as Source)
    .fetch_one(connection.as_ref())
    .instrument(query_span.clone())
    .await
    {
        Ok(row) => {
            tracing::info!("Found Movie record in the database");
            (
                row.get("torrent_id"),
                row.get::<Uuid, &str>("id"),
                row.get("movie_path"),
            )
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::error!("Movie not found in the database");
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
              "error" : "Database Error"
            }));
        }
    };

    let torrent_client = RqbitWrapper::default();
    match torrent_client.delete_torrent(torrent_id, movie_path).await {
        Ok(_) => {}
        Err(err) => {
            return HttpResponse::BadRequest().json(json!({
              "error": err
            }));
        }
    }

    match sqlx::query(
        r#"
        DELETE FROM movie_torrent WHERE id = $1
      "#,
    )
    .bind(record_id)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("Movie record deleted from the database");
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            HttpResponse::BadRequest().json(json!({
              "error": "something went wrong"
            }))
        }
    }
}
