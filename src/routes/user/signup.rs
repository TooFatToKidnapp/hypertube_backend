use super::util::{validate_password, validate_user_name};
use crate::routes::generate_token;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Uuid;
use sqlx::Error::Database;
use sqlx::PgPool;
use tracing::Instrument;
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
    let user_id = Uuid::new_v4();
    let query_result = sqlx::query!(
        r#"
			INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
		"#,
        user_id,
        body.username,
        body.email,
        password_hash,
        Utc::now(),
        Utc::now()
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span)
    .await;

    let user = match query_result {
        Err(err) => match err {
            Database(err)
                if err
                    .message()
                    .contains("duplicate key value violates unique constraint")
                    && err.message().contains("email") =>
            {
                tracing::error!("Email already exists in the database");
                return HttpResponse::BadRequest()
                    .json(json!({
                        "error": "Email already exists"
                    }));
            }
            _ => {
                tracing::error!("Failed to create user {:?}", err);
                return HttpResponse::InternalServerError()
                    .json(json!({
                        "error": "Failed to create user"
                    }));
            }
        },
        Ok(user) => user,
    };

    tracing::info!("Generating user token");

    let token_result = generate_token(user_id.to_string());
    match token_result {
        Ok(token) => {
            tracing::info!("successful Login");
            HttpResponse::Ok().json(json!({
                "data" : {
                    "token": token,
                    "email": user.email,
                    "created_at": user.created_at.to_string(),
                    "updated_at": user.updated_at.to_string(),
                    "username" : user.username,
                }
            }))
        }
        Err(_) => {
            tracing::error!("Error Generating token");
            HttpResponse::Unauthorized().json(json!({
                "Error": "Something went wrong"
            }))
        }
    }
}
