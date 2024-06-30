use super::util::{validate_password, validate_user_name};
use crate::util::ResponseMessage;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Uuid;
use sqlx::Error::Database;
use sqlx::PgPool;
use tracing::{info, Instrument};
use validator::Validate;

#[derive(Deserialize, Debug, Validate)]
pub struct CreateUserRequest {
    #[validate(custom(function = "validate_user_name"))]
    username: String,
    #[validate(email(message = "Not a valid email"))]
    email: String,
    #[validate(custom(function = "validate_password"))]
    password: String,
}

pub async fn user_signup(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    tracing::info!("Got request body: {:#?}", body);
    let is_valid = body.validate();
    if let Err(error) = is_valid {
        let source = error.field_errors();
        for i in source.iter() {
            for err in i.1.iter() {
                if let Some(message) = err.message.as_ref() {
                    tracing::error!("Error: {}", message.as_ref());
                    return HttpResponse::BadRequest().json(json!({
                        "Error" : message.as_ref()
                    }));
                }
            }
        }
        return HttpResponse::BadRequest().finish();
    }
    let query_span = tracing::info_span!("Saving new user details in the database", ?body);
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(body.password.as_bytes(), &salt);
    let password_hash = match password_hash {
        Ok(hash) => {
            tracing::info!("Password hashed successfully");
            hash.to_string()
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "Error": e.to_string()
            }));
        }
    };
    let result = sqlx::query!(
        r#"
			INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6)
		"#,
        Uuid::new_v4(),
        body.username,
        body.email,
        password_hash,
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
        Err(err) => match err {
            Database(err)
                if err
                    .message()
                    .contains("duplicate key value violates unique constraint")
                    && err.message().contains("email") =>
            {
                tracing::error!("Email already exists in the database");
                HttpResponse::BadRequest().json(ResponseMessage::new("Email already exists"))
            }
            _ => {
                tracing::error!("Failed to create user {:?}", err);
                HttpResponse::InternalServerError()
                    .json(ResponseMessage::new("Failed to create user"))
            }
        },
    }
}
