use std::borrow::Cow;

use actix_web::{
    cookie::{
        time::{Duration, OffsetDateTime},
        Cookie, Expiration, SameSite,
    },
    web::{self, get, patch, post},
    Scope,
};
use chrono::Utc;
use regex::Regex;
use sqlx::types::Uuid;
use sqlx::PgPool;
use validator::ValidationError;

use crate::middleware::{Authentication, User};

use super::{
    get_user, profile_password_reset, sign_out_user, update_profile_information,
    upload_user_profile_image, user_login, user_signup,
};

pub fn user_source(db_pool: &PgPool) -> Scope {
    web::scope("/users")
        .route("/sign-up", web::post().to(user_signup))
        .route("/login", post().to(user_login))
        .route(
            "/password/update",
            post()
                .to(profile_password_reset)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/upload",
            post()
                .to(upload_user_profile_image)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/sign-out",
            get()
                .to(sign_out_user)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/update",
            patch()
                .to(update_profile_information)
                .wrap(Authentication::new(db_pool.clone())),
        )
        .route(
            "/{id}",
            get()
                .to(get_user)
                .wrap(Authentication::new(db_pool.clone())),
        )
}

const CHECK_FOR_UPPERCASE: &str = ".*[A-Z].*";
const CHECK_FOR_LOWERCASE: &str = ".*[a-z].*";
const CHECK_FOR_NUMBER: &str = ".*[0-9].*";
const CHECK_FOR_SPECIAL_CHARACTER: &str = r".*[^A-Za-z0-9].*";
const FORBIDDEN_CHARACTERS: &[char] = &['/', '(', ')', '"', '<', '>', '\\', '{', '}', '\''];

pub fn validate_password(password: &str) -> Result<(), ValidationError> {
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

pub fn validate_user_name(user_name: &str) -> Result<(), ValidationError> {
    if user_name.len() > 50 {
        return Err(ValidationError::new("User name length error")
            .with_message(Cow::from("User name must be less then 50 characters")));
    }
    if user_name.is_empty() {
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

pub fn validate_user_first_name(first_name: &str) -> Result<(), ValidationError> {
    if first_name.len() > 50 {
        return Err(ValidationError::new("User first name length error")
            .with_message(Cow::from("User first name must be less then 50 characters")));
    }
    if first_name.is_empty() {
        return Err(ValidationError::new("User first name length error")
            .with_message(Cow::from("User first name can't be empty")));
    }
    if first_name.trim().is_empty() {
        return Err(
            ValidationError::new("User first name content error").with_message(Cow::from(
                "User first name must contain at least 1 non-whitespace character",
            )),
        );
    }
    if first_name
        .chars()
        .any(|c| FORBIDDEN_CHARACTERS.contains(&c))
    {
        return Err(
            ValidationError::new("User first name content error").with_message(Cow::from(
                "User first name cannot contain any of the following characters [/, (, ), \", <, >, \\, {, }, ']",
            )),
        );
    }
    Ok(())
}

pub fn validate_user_last_name(last_name: &str) -> Result<(), ValidationError> {
    if last_name.len() > 50 {
        return Err(ValidationError::new("User last name length error")
            .with_message(Cow::from("User last name must be less then 50 characters")));
    }
    if last_name.is_empty() {
        return Err(ValidationError::new("User last name length error")
            .with_message(Cow::from("User last name can't be empty")));
    }
    if last_name.trim().is_empty() {
        return Err(
            ValidationError::new("User last name content error").with_message(Cow::from(
                "User last name must contain at least 1 non-whitespace character",
            )),
        );
    }
    if last_name.chars().any(|c| FORBIDDEN_CHARACTERS.contains(&c)) {
        return Err(
            ValidationError::new("User last name content error").with_message(Cow::from(
                "User last name cannot contain any of the following characters [/, (, ), \", <, >, \\, {, }, ']",
            )),
        );
    }
    Ok(())
}

pub async fn create_session(
    connection: &PgPool,
    user: User,
) -> Result<Cookie, Box<dyn std::error::Error>> {
    let session_query_result = sqlx::query!(
        r#"
            INSERT INTO sessions (id, user_id, session_data, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
        "#,
        Uuid::new_v4(),
        user.id,
        serde_json::json!({
            "first_name": user.first_name,
            "last_name": user.last_name,
            "username": user.username,
            "email": user.email,
            "created_at": user.created_at,
            "updated_at": user.updated_at
        }),
        Utc::now(),
        Utc::now() + chrono::Duration::days(7)
    )
    .fetch_one(connection)
    .await;

    let session_id = match session_query_result {
        Ok(session) => {
            tracing::info!("Session Created");
            session.id.to_string()
        }
        Err(err) => {
            tracing::error!("database error {}", err);
            return Err(Box::new(err));
        }
    };
    let cookie = Cookie::build("session", session_id)
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .expires(Expiration::DateTime(
            OffsetDateTime::now_utc() + Duration::days(7),
        ))
        .finish();
    Ok(cookie)
}

pub fn validate_uuid(id: &str) -> Result<(), ValidationError> {
    match Uuid::parse_str(id) {
        Ok(parsed_uuid) => {
            if parsed_uuid.get_version_num() == 4 {
                return Ok(());
            }
            Err(ValidationError::new("Invalid verification id")
                .with_message(Cow::from("Invalid UUID version")))
        }
        Err(_) => Err(ValidationError::new("Invalid verification id")
            .with_message(Cow::from("Not a valid UUID"))),
    }
}
