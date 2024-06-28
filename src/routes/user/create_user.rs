use std::error::Error;

use crate::util::ResponseMessage;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::{Connection, PgConnection, PgPool};
use tracing::Instrument;
use validator::{Validate, ValidationError};

const CHECK_FOR_UPPERCASE: &str = ".*[A-Z].*";
const CHECK_FOR_LOWERCASE: &str = ".*[a-z].*";
const CHECK_FOR_NUMBER: &str = ".*[0-9].*";
const CHECK_FOR_SPECIAL_CHARACTER: &str = r".*[^A-Za-z0-9].*";

// fn validate_unique_email(user_email: &str) -> Result<(), ValidationError> {
//     let rt = tokio::runtime::Runtime::new().expect("Can't initialize the tokio runtime");
//     let query_span = tracing::info_span!(
//         "Checking if email already exists in the database",
//         ?user_email
//     );

//     let configuration = crate::configuration::get_configuration("configuration")
//         .expect("Failed to locate configuration.json file");
//     let connection_url = format!(
//         "postgresql://{}:{}@{}/{}",
//         configuration.database.user_name,
//         configuration.database.password,
//         configuration.database.host,
//         configuration.database.database_name
//     );

//     let result = rt.spawn_blocking( async  move  {
//         let connection = PgConnection::connect(connection_url.as_str())
//             .await
//             .expect("Failed to connect to Postgres for cleanup");

//         return sqlx::query!(
//             r#"
//             SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)
//             "#,
//             user_email
//         )
//         .fetch_one(&mut connection)
//         .instrument(query_span)
//         .await;
//     });

//     match result {
//         Ok(row) => {
//             if let Some(_) = row.exists {
//                 Err(ValidationError::new("User already exists with that email"))
//             } else {
//                 Ok(())
//             }
//         }
//         Err(_) => Err(ValidationError::new("Something went wrong")),
//     }
// }
use std::borrow::Cow;

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

#[derive(Deserialize, Debug, Validate)]
pub struct CreateUserRequest {
    #[validate(length(
        min = 1,
        max = 50,
        message = "The user name must be between 1 and 50 characters in length."
    ))]
    username: String,
    #[validate(email(message = "Not a valid email"))]
    // #[validate(email, custom(function = "validate_unique_email"))]
    email: String,
    #[validate(custom(function = "validate_password"))]
    password: String,
}

pub async fn create_user(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    let is_valid = body.validate();
    println!("\nis_valid\n {:#?}", is_valid);
    if let Err(error) = is_valid {
        let source = error.field_errors();
        for i in source.iter() {
            for err in i.1.iter() {
                println!("err: {}", err);
                if let Some(message) = err.message.as_ref() {
                    return HttpResponse::BadRequest().json(ResponseMessage {
                        message: message.as_ref().to_string()
                    });
                }
            }
        }
        return HttpResponse::BadRequest().finish();
    }
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
