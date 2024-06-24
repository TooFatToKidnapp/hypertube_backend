use crate::util::ResponseMessage;
use chrono::Utc;
use sqlx::types::Uuid;

use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::Instrument;

#[derive(Deserialize, Debug)]
pub struct CreateUserRequest {
    username: String,
    email: String,
    password: String,
}

pub async fn create_user(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Saving new subscriber details in the database", ?body);
    let result = sqlx::query!(
        r#"
			INSERT INTO users (id, username, email, password, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6)
		"#,
        Uuid::new_v4(),
        body.username,
        body.email,
        body.password,
        Utc::now(),
        Utc::now()
    )
    .execute(connection.get_ref())
    .instrument(query_span)
    .await;

    match result {
        Ok(_res) => {
            tracing::info!("User created successfully");
            HttpResponse::Ok().json(ResponseMessage::new("User created successfully"))
        }
        Err(err) => {
            tracing::error!("Failed to create user {:?}", err);
            HttpResponse::InternalServerError().json(ResponseMessage::new("Failed to create user"))
        }
    }
}
