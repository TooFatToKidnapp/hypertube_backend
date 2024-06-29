use crate::util::ResponseMessage;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use sha3::Digest;
use sqlx::types::Uuid;
use sqlx::Error::Database;
use sqlx::PgPool;
use std::borrow::Cow;
use tracing::{info, Instrument};
use validator::{Validate, ValidationError};

const CHECK_FOR_UPPERCASE: &str = ".*[A-Z].*";
const CHECK_FOR_LOWERCASE: &str = ".*[a-z].*";
const CHECK_FOR_NUMBER: &str = ".*[0-9].*";
const CHECK_FOR_SPECIAL_CHARACTER: &str = r".*[^A-Za-z0-9].*";
const FORBIDDEN_CHARACTERS: &[char] = &['/', '(', ')', '"', '<', '>', '\\', '{', '}', '\''];

fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("Password length")
            .with_message(Cow::from("Password must be at least 8 characters long")));
    }
    let uppercase_pattern = Regex::new(CHECK_FOR_UPPERCASE).unwrap();
    let lowercase_pattern = Regex::new(CHECK_FOR_LOWERCASE).unwrap();
    let number_pattern = Regex::new(CHECK_FOR_NUMBER).unwrap();
    let special_char_pattern = Regex::new(CHECK_FOR_SPECIAL_CHARACTER).unwrap();

    if !uppercase_pattern.is_match(password) {
        return Err(
            ValidationError::new("Password missing UpperCase").with_message(Cow::from(
                "Password must contain at least one uppercase letter",
            )),
        );
    }
    if !lowercase_pattern.is_match(password) {
        return Err(
            ValidationError::new("Password missing  LowerCase").with_message(Cow::from(
                "Password must contain at least one lowercase letter",
            )),
        );
    }
    if !number_pattern.is_match(password) {
        return Err(ValidationError::new("Password missing Number")
            .with_message(Cow::from("Password must contain at least one number")));
    }
    if !special_char_pattern.is_match(password) {
        return Err(
            ValidationError::new("Password missing Special Char").with_message(Cow::from(
                "Password must contain at least one special character",
            )),
        );
    }
    Ok(())
}

fn validate_user_name(user_name: &str) -> Result<(), ValidationError> {
    if user_name.len() > 50 {
        return Err(ValidationError::new("User name length error")
            .with_message(Cow::from("User name must be less then 50 characters")));
    }
    if user_name.len() < 1 {
        return Err(ValidationError::new("User name length error")
            .with_message(Cow::from("User name can't be empty")));
    }
    if user_name.trim().is_empty() {
        return Err(
            ValidationError::new("User name content error").with_message(Cow::from(
                "User name must contain at least 1 non-whitespace character",
            )),
        );
    }
    if user_name.chars().any(|c| FORBIDDEN_CHARACTERS.contains(&c)) {
        return Err(
            ValidationError::new("User name content error").with_message(Cow::from(
                "User name cannot contain any of the following characters [/, (, ), \", <, >, \\, {, }, ']",
            )),
        );
    }
    Ok(())
}

#[derive(Deserialize, Debug, Validate)]
pub struct CreateUserRequest {
    #[validate(custom(function = "validate_user_name"))]
    username: String,
    #[validate(email(message = "Not a valid email"))]
    email: String,
    #[validate(custom(function = "validate_password"))]
    password: String,
}

pub async fn create_user(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    tracing::info!("Got request body: {:#?}", body);
    let is_valid = body.validate();
    if let Err(error) = is_valid {
        let source = error.field_errors();
        for i in source.iter() {
            for err in i.1.iter() {
                if let Some(message) = err.message.as_ref() {
                    tracing::error!("Error: {}", message.as_ref());
                    return HttpResponse::BadRequest().json(ResponseMessage::new(message.as_ref()));
                }
            }
        }
        return HttpResponse::BadRequest().finish();
    }
    let query_span = tracing::info_span!("Saving new subscriber details in the database", ?body);
    let password_hash = sha3::Sha3_256::digest(body.password.as_bytes());
    let password_hash = format!("{:x}", password_hash);
    info!("Hashed password {}", password_hash);
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
