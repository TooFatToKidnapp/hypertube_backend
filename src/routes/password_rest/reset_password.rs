use crate::routes::validate_password;
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
use sqlx::PgPool;
use std::borrow::Cow;
use tracing::Instrument;
use uuid::Uuid;
use validator::Validate;
use validator::ValidationError;

fn validate_verification_id(verification_id: &str) -> Result<(), ValidationError> {
    match Uuid::parse_str(verification_id) {
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

#[derive(Deserialize, Validate)]
pub struct UpdatePassword {
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,
    #[validate(email(message = "Not a valid email"))]
    pub email: String,
    #[validate(custom(function = "validate_verification_id"))]
    pub verification_id: String,
}

pub async fn reset_password(body: Json<UpdatePassword>, connection: Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Update User Password");
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

    let query_res = sqlx::query!(
        r#"
            SELECT * from password_verification_code WHERE id = $1
        "#,
        Uuid::parse_str(body.verification_id.as_str()).unwrap()
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    match query_res {
        Ok(res) => {
            tracing::info!("found verification record");
            if !res.is_validated {
                tracing::info!("record not validated");
                return HttpResponse::BadRequest().json(json!({
                    "message": "not yet verified"
                }));
            }
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }

    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(body.new_password.as_bytes(), &salt);
    let password_hash = match password_hash {
        Ok(hash) => {
            tracing::info!("Password hashed successfully");
            hash.to_string()
        }
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "Error": e.to_string()
            }));
        }
    };

    let query_res = sqlx::query!(
        r#"
            UPDATE users SET password_hash = $1, updated_at = $2 WHERE email = $3
        "#,
        password_hash,
        Utc::now(),
        body.email
    )
    .execute(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    match query_res {
        Ok(_) => tracing::info!("User Password updated successfully"),
        Err(err) => {
            tracing::error!("database error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };

    let delete_query_res = sqlx::query(
        r#"
            DELETE FROM password_verification_code WHERE id = $1
        "#,
    )
    .bind(Uuid::parse_str(body.verification_id.as_str()).unwrap())
    .execute(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    match delete_query_res {
        Ok(rows_affected) => tracing::info!("Number of rows deleted = {:#?}", rows_affected),
        Err(err) => {
            tracing::error!("Failed to delete previous verification codes {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }
    tracing::info!("User completed the password update process");
    HttpResponse::Ok().json(json!({
        "message": "password updated"
    }))
}
