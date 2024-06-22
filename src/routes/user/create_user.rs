use actix_web::{HttpResponse, web::{Data, Json}};
use sqlx::PgPool;
use serde::Deserialize;
use crate::util::ResponseMessage;

#[derive(Deserialize)]
struct CreateUserRequest {
		username: String,
		email: String,
		password: String,
}

async fn create_user(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
		let result = sqlx::query!(
				r#"
						INSERT INTO users (username, email, password)
						VALUES ($1, $2, $3)
				"#,
				body.username,
				body.email,
				body.password
		)
		.execute(connection.get_ref())
		.await;

		match result {
				Ok(_) => HttpResponse::Ok().json(ResponseMessage::new("User created successfully")),
				Err(_) => HttpResponse::InternalServerError().json(ResponseMessage::new("Failed to create user")),
		}
}
