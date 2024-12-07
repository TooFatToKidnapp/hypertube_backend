use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing::Instrument;
use uuid::Uuid;

use crate::routes::Source;

#[derive(Deserialize, Debug)]
pub struct MovieIdentifier {
    pub movie_id: i32,
    pub source: Source,
}

#[derive(Deserialize, Debug)]
pub struct PageInfo {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

pub async fn get_movie_comments(
    connection: Data<PgPool>,
    movie_info: Path<MovieIdentifier>,
    page_info: Query<PageInfo>,
) -> HttpResponse {
    let movie_info = movie_info.into_inner();
    let query_span = tracing::info_span!("Get movie comments");

    let page = page_info.page.unwrap_or_default();
    let page_size = if page_info.page_size.is_some() {
        let size = page_info.page_size.unwrap();
        if size == 0 {
            10
        } else {
            size
        }
    } else {
        10
    };

    let rows = match sqlx::query(
        r#"
            SELECT uc.id as comment_id, u.id as user_id, uc.*, u.*
            FROM user_comments uc
            JOIN users u ON uc.user_id = u.id
            WHERE uc.movie_id = $1 AND uc.movie_source = $2
            ORDER BY uc.created_at DESC
            LIMIT $3 OFFSET $4
        "#,
    )
    .bind(movie_info.movie_id)
    .bind(movie_info.source.clone() as Source)
    .bind(page_size as i32)
    .bind((page * page_size) as i32)
    .fetch_all(connection.as_ref())
    .instrument(query_span.clone())
    .await
    {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!("Database error {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Something went wrong"
            }));
        }
    };

    let movie_comments_max_count: i64 = match sqlx::query_scalar(
        r#"
            SELECT COUNT(*)
            FROM user_comments
            WHERE movie_id = $1 AND movie_source = $2
        "#,
    )
    .bind(movie_info.movie_id)
    .bind(movie_info.source as Source)
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(count) => count,
        Err(err) => {
            tracing::error!("Database error {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Something went wrong"
            }));
        }
    };

    let mut comments = Vec::<serde_json::Value>::new();
    for row in rows.iter() {
        let user_id: Uuid = row.get("user_id");
        let username: String = row.get("username");
        let profile_picture_url: Option<String> = row.get("profile_picture_url");

        let comment_id: Uuid = row.get("comment_id");
        let comment_created_at: DateTime<Utc> = row.get("created_at");
        let comment: String = row.get("comment");

        comments.push(json!({
            "user_info" : {
                "user_id": user_id.to_string(),
                "username": username,
                "profile_picture": profile_picture_url,
            },
            "comment_info": {
                "comment_id": comment_id.to_string(),
                "comment" : comment,
                "created_at": comment_created_at
            }
        }));
    }

    HttpResponse::Ok().json(json!({
        "data": {
            "comments": comments,
            "max_comments": movie_comments_max_count,
            "page_count": movie_comments_max_count / page_size as i64
        }
    }))
}
