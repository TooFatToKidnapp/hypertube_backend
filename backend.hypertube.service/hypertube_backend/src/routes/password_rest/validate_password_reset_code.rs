use crate::middleware::User;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use chrono::Utc;

use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tracing::Instrument;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct ValidateResetCode {
    pub code: String,
    // #[validate(email(message = "Not a valid email"))]
    pub username: String,
}

pub async fn validate_password_reset_code(
    body: Json<ValidateResetCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Validate password reset code");
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
            SELECT * FROM users WHERE username = $1
        "#,
        body.username
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    let user = match query_res {
        Ok(user) => User {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            username: user.username,
            email: user.email,
            image_url: user.profile_picture_url,
            created_at: user.created_at.to_string(),
            updated_at: user.updated_at.to_string(),
            session_id: None,
        },
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("User with username {} not found in database", body.username);
            return HttpResponse::NotFound().json(json!({
                "message": "User not found"
            }));
        }
        Err(err) => {
            tracing::error!("Database error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };

    let query_res = sqlx::query!(
        r#"
            SELECT * FROM password_verification_code WHERE username = $1 ORDER BY created_at DESC LIMIT 1
        "#,
        user.username
    ).fetch_one(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    let verification_record = match query_res {
        Ok(res) => {
            tracing::info!("Got verification row");
            res
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error" : "something went wrong"
            }));
        }
    };

    if verification_record.code != body.code {
        tracing::info!("Verification code is NOT correct");
        return HttpResponse::BadRequest().json(json!({
            "message": "Wrong verification code"
        }));
    }

    if verification_record.expires_at < Utc::now() {
        tracing::info!("Verification code expired");
        return HttpResponse::BadRequest().json(json!({
            "message": "verification code expired"
        }));
    }

    tracing::info!("Correct verification code");

    let query_res = sqlx::query!(
        r#"
            UPDATE password_verification_code SET is_validated = $1 WHERE id = $2
        "#,
        true,
        verification_record.id
    )
    .execute(connection.as_ref())
    .instrument(query_span)
    .await;

    match query_res {
        Ok(_) => tracing::info!("verification record updated"),
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }

    HttpResponse::Ok().json(json!({
        "verification_id": verification_record.id.to_string(),
    }))
}
