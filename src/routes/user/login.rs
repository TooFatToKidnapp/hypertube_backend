use crate::{middleware::User, routes::create_session};
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
            SELECT * FROM users WHERE email = $1 AND password_hash IS NOT NULL
        "#,
        body.email
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span)
    .await;

    let user = match result {
        Ok(user) => {
            tracing::info!("got user form database {:#?}", user);
            user
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::error!("User not found in the database");
            return HttpResponse::Unauthorized().json(json!({
                "Error": "Invalid email or password"
            }));
        }
        Err(err) => {
            tracing::error!("Error getting user from database {}", err);
            return HttpResponse::InternalServerError().json(json!({
                "Error": "something went wrong"
            }));
        }
    };
    let password = user.password_hash.unwrap();
    let result = PasswordHash::new(password.as_str());
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

    let user = User {
        id: user.id,
        first_name: user.first_name,
        last_name: user.last_name,
        image_url: user.profile_picture_url,
        email: user.email,
        created_at: user.created_at.to_string(),
        updated_at: user.updated_at.to_string(),
        username: user.username,
    };
    let session_result = create_session(connection.as_ref(), user.clone()).await;
    if session_result.is_err() {
        tracing::error!(
            "Failed to generate user session  {}",
            session_result.unwrap_err()
        );
        return HttpResponse::InternalServerError().json(json!({
            "error": "something went wrong"
        }));
    }
    HttpResponse::Ok()
        .cookie(session_result.unwrap())
        .json(json!({
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
