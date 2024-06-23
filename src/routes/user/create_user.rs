use crate::util::ResponseMessage;
use chrono::Utc;
// use uuid::Uuid;
use sqlx::types::Uuid; // Add this line

use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize, Debug)]
pub struct CreateUserRequest {
    username: String,
    email: String,
    password: String,
}

pub async fn create_user(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    // let id = uuid::Uuid::new_v4();
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
    .await;

    match result {
        Ok(res) => {
            println!("db res: {:?}", res);
            HttpResponse::Ok().json(ResponseMessage::new("User created successfully"))
        }
        Err(_) => {
            HttpResponse::InternalServerError().json(ResponseMessage::new("Failed to create user"))
        }
    }
}
