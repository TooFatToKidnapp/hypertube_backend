use super::util::validate_password;
use crate::middleware::User;
use actix_web::{
    web::{Data, Json},
    HttpMessage, HttpRequest, HttpResponse,
};
use argon2::PasswordHasher;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::rc::Rc;
use tracing::Instrument;
use validator::Validate;

#[derive(Deserialize, Validate, Debug)]
pub struct ResetPassword {
    // #[validate(custom(function = "validate_password"))]
    // pub old_password: String,
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,
}

pub async fn profile_password_set(
    req: HttpRequest,
    body: Json<ResetPassword>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("User Password reset", ?body);

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

    let user_email = {
        let extension = req.extensions();
        let user_option = extension.get::<Rc<User>>();
        match user_option {
            Some(user) => user.email.clone(),
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "No user info in request payload"
                }));
            }
        }
    };

    let query_result = sqlx::query!(
        r#"
			SELECT * FROM users WHERE email = $1
		"#,
        user_email
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span.clone())
    .await;

    let user = match query_result {
        Ok(record) => record,
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("User not found in the database");
            return HttpResponse::NotFound().json(json!({
                "message": "User not found in the database"
            }));
        }
        Err(err) => {
            tracing::error!("Database Error {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };
    // let password_hash = user.password_hash.unwrap();
    // let password_hash_result = PasswordHash::new(password_hash.as_str());
    // let password_hash = match password_hash_result {
    //     Ok(hash) => {
    //         tracing::info!("Got password hash");
    //         hash
    //     }
    //     Err(err) => {
    //         tracing::error!("Can't get password hash {}", err);
    //         return HttpResponse::BadRequest().json(json!({
    //             "error": "Can't get password hash"
    //         }));
    //     }
    // };

    // let password_validation_result =
    //     Argon2::default().verify_password(body.old_password.as_bytes(), &password_hash);

    // match password_validation_result {
    //     Ok(()) => tracing::info!("Correct Password"),
    //     Err(err) => {
    //         tracing::error!("Wrong Password {}", err);
    //         return HttpResponse::BadRequest().json(json!({
    //             "error": "Wrong password"
    //         }));
    //     }
    // };

    // if body.new_password == body.old_password {
    //     tracing::error!("New password must be different from the old one");
    //     return HttpResponse::BadRequest().json(json!({
    //         "error": "New password must be different from the old one"
    //     }));
    // }

    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash_result = argon2.hash_password(body.new_password.as_bytes(), &salt);
    let password_hash = match password_hash_result {
        Ok(hash) => {
            tracing::info!("New Password hashed successfully");
            hash.to_string()
        }
        Err(err) => {
            tracing::error!("Can't get new password hash {}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Can't get password hash"
            }));
        }
    };

    let query_result = sqlx::query(
        r#"
				UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3
			"#,
    )
    .bind(password_hash)
    .bind(Utc::now())
    .bind(user.id)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await;

    match query_result {
        Ok(_) => {
            tracing::info!("Password updated Successfully");
            HttpResponse::Ok().json(json!({
                "message": "Password updated Successfully"
            }))
        }
        Err(err) => {
            tracing::error!("Database Error {}", err);
            HttpResponse::BadRequest().json(json!({
                    "Error": "something went wrong"
            }))
        }
    }
}
