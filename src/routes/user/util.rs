use std::borrow::Cow;

use actix_web::{
    web::{self, get, post},
    Scope,
};
use regex::Regex;
use validator::ValidationError;

use super::{get_user, user_login, user_signup};

pub fn user_source() -> Scope {
    web::scope("/user")
        .route("/sign-up", web::post().to(user_signup))
        .route("/login", post().to(user_login))
        .route("/", get().to(get_user))
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
