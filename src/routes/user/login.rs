use crate::routes::generate_token;

use super::util::validate_password;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tracing::Instrument;
use validator::Validate;

#[derive(Deserialize, Debug, Validate)]
pub struct UserData {
    #[validate(email(message = "Not a valid email"))]
    pub email: String,
    #[validate(custom(function = "validate_password"))]
    pub password: String,
}

pub async fn user_login(body: Json<UserData>, connection: Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Handel User Login", ?body);

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

    let result = sqlx::query!(
        r#"
      SELECT * FROM users WHERE email = $1
    "#,
        body.email,
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span)
    .await;

    let user = match result {
        Ok(user) => {
            tracing::info!("got user form database {:#?}", user);
            user
        }
        Err(err) => {
            tracing::error!("Error getting user from database {}", err);
            return HttpResponse::Unauthorized().json(json!({
                "Error": "Invalid email or password"
            }));
        }
    };
    let result = PasswordHash::new(user.password_hash.as_str());
    let parsed_hash = match result {
        Ok(hash) => {
            tracing::info!("parsed the hashed password");
            hash
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "Error": "Something went wrong"
            }));
        }
    };
    let result = Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_ok();
    match result {
        true => {}
        false => {
            tracing::error!("Wrong Password");
            return HttpResponse::Unauthorized().json(json!({
                "Error": "Invalid email or password"
            }));
        }
    };
    tracing::info!("Password is correct");

    let token_result = generate_token(user.id.to_string());
    match token_result {
        Ok(token) => {
            tracing::info!("successful Login");
            HttpResponse::Ok().json(json!({
                "data" : {
                    "id": user.id.to_string(),
                    "email": user.email,
                    "username": user.username,
                    "created_at": user.created_at.to_string(),
                    "updated_at": user.updated_at.to_string(),
                    "token": token
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
