use super::util::{
    validate_password, validate_user_first_name, validate_user_last_name, validate_user_name,
};
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::Cookie;
use actix_web::{
    cookie::{Expiration, SameSite},
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
    #[validate(custom(function = "validate_user_first_name"))]
    first_name: String,
    #[validate(custom(function = "validate_user_last_name"))]
    last_name: String,
    #[validate(custom(function = "validate_user_name"))]
    username: String,
    #[validate(email(message = "Not a valid email"))]
    email: String,
    #[validate(custom(function = "validate_password"))]
    password: String,
}

pub async fn user_signup(body: Json<CreateUserRequest>, connection: Data<PgPool>) -> HttpResponse {
    let query_span = tracing::info_span!("Saving new user details in the database", ?body);
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
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(body.password.as_bytes(), &salt);
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

    let transaction_result = connection.begin().await;
    let mut transaction = match transaction_result {
        Ok(transaction) => transaction,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "Error": e.to_string()
            }));
        }
    };

    let user_id = Uuid::new_v4();
    let query_result = sqlx::query!(
        r#"
			INSERT INTO users (id, username, email, first_name, last_name, password_hash, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
		"#,
        user_id,
        body.username,
        body.email,
        body.first_name,
        body.last_name,
        password_hash,
        Utc::now(),
        Utc::now()
    )
    .fetch_one(&mut *transaction)
    .instrument(query_span.clone())
    .await;

    let user = match query_result {
        Err(err) => match err {
            Database(err)
                if err
                    .message()
                    .contains("duplicate key value violates unique constraint")
                    && err.message().contains("email") =>
            {
                let res = transaction.rollback().await;
                if res.is_err() {
                    tracing::error!("failed to rollback changes in the database");
                    return HttpResponse::BadRequest().json(json!({
                        "error": "failed to rollback changes in the database"
                    }));
                }
                tracing::error!("Email already exists in the database");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Email already exists"
                }));
            }
            _ => {
                let res = transaction.rollback().await;
                if res.is_err() {
                    tracing::error!("failed to rollback changes in the database");
                    return HttpResponse::BadRequest().json(json!({
                        "error": "failed to rollback changes in the database"
                    }));
                }
                tracing::error!("Failed to create user {:?}", err);
                return HttpResponse::BadRequest().json(json!({
                    "error": "Failed to create user"
                }));
            }
        },
        Ok(user) => user,
    };

    let session_query_result = sqlx::query!(
        r#"
            INSERT INTO sessions (id, user_id, session_data, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
        "#,
        Uuid::new_v4(),
        user_id,
        serde_json::json!({
            "username": user.username,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "email": user.email,
            "created_at": user.created_at,
            "updated_at": user.updated_at
        }),
        Utc::now(),
        Utc::now() + chrono::Duration::days(7)
    )
    .fetch_one(&mut *transaction)
    .instrument(query_span.clone())
    .await;
    let session_id = match session_query_result {
        Ok(session) => {
            tracing::info!("Session Created");
            let res = transaction.commit().await;
            if res.is_err() {
                tracing::error!("failed to write changes to the database");
                return HttpResponse::BadRequest().json(json!({
                    "error": "failed to write changes to the database"
                }));
            }
            session.id.to_string()
        }
        Err(err) => {
            let res = transaction.rollback().await;
            if res.is_err() {
                tracing::error!("failed to rollback changes in the database");
                return HttpResponse::BadRequest().json(json!({
                    "error": "failed to rollback changes in the database"
                }));
            }
            tracing::error!("database error {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "database error"
            }));
        }
    };

    let cookie = Cookie::build("session", session_id)
        .secure(true)
        .http_only(true)
        .same_site(SameSite::None)
        .path("/")
        .expires(Expiration::DateTime(
            OffsetDateTime::now_utc() + Duration::days(7),
        ))
        .finish();

    tracing::info!("successful Login");
    HttpResponse::Ok().cookie(cookie).json(json!({
        "data" : {
            "id": user.id.to_string(),
            "email": user.email,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "created_at": user.created_at.to_string(),
            "updated_at": user.updated_at.to_string(),
            "username" : user.username,
        }
    }))
}
