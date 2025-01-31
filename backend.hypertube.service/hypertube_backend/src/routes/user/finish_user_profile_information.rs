use std::rc::Rc;

use crate::middleware::User;
use actix_web::{
    web::{Data, Json},
    HttpMessage, HttpRequest, HttpResponse,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::{postgres::PgDatabaseError, PgPool};
use tracing::Instrument;
use validator::Validate;

use super::util::{validate_user_first_name, validate_user_last_name, validate_user_name};
#[derive(Validate, Deserialize, Debug)]
pub struct NewProfileInformation {
    #[validate(custom(function = "validate_user_first_name"))]
    first_name: Option<String>,
    #[validate(custom(function = "validate_user_last_name"))]
    last_name: Option<String>,
    #[validate(custom(function = "validate_user_name"))]
    username: Option<String>,
}

pub async fn finish_profile_information(
    connection: Data<PgPool>,
    body: Json<NewProfileInformation>,
    req: HttpRequest,
) -> HttpResponse {
    let query_span = tracing::info_span!("Updating user information", ?body);
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

    if body.first_name.is_none()
        || body.last_name.is_none()
        || body.username.is_none()
    {
        return HttpResponse::BadRequest().json(json!({
            "error": "missing fields"
        }));
    }

    let password_is_set = {
        let extension = req.extensions();
        let user_option = extension.get::<Rc<User>>();
        match user_option {
            Some(user) => user.password_is_set.clone(),
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "No user info in request payload"
                }));
            }
        }
    };

    if !password_is_set {
        return HttpResponse::BadRequest().json(json!({
            "error": "you should set your password before updating your profile"
        }));
    }


    let user_id = {
        let extension = req.extensions();
        match extension.get::<Rc<User>>() {
            Some(user) => user.id,
            None => {
                tracing::info!("User field not found in req object");
                return HttpResponse::NotFound().json(json!({
                        "error": "user not found"
                }));
            }
        }
    };

    let mut transaction = match connection.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "Error": e.to_string()
            }));
        }
    };

    let query_result = sqlx::query!(
        r#"
				SELECT * FROM users WHERE id = $1
		"#,
        user_id
    )
    .fetch_one(&mut *transaction)
    .instrument(query_span.clone())
    .await;

    let user = match query_result {
        Ok(user) => user,
        Err(err) => {
            let _ = transaction.rollback().await;
            tracing::error!("Error getting user from database {}", err);
            return HttpResponse::Unauthorized().json(json!({
                    "Error" : "database error"
            }));
        }
    };

    if user.password_hash.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Must set a password first before updating your profile"
        }));
    }

    // if body.email.is_some() {
    //     tracing::info!("Updating user email");
    //     let new_email = body.email.clone().unwrap();
    //     // let query_res = sqlx::query(
    //     //     r#"
	// 	// 		SELECT id FROM users WHERE email = $1
	// 	// 	"#,
    //     // )
    //     // .bind(new_email.as_str())
    //     // .fetch_optional(&mut *transaction)
    //     // .instrument(query_span.clone())
    //     // .await;

    //     // match query_res {
    //     //     Ok(Some(_)) => {
    //     //         tracing::error!("Email already in use");
    //     //         let _ = transaction.rollback().await;
    //     //         return HttpResponse::BadRequest().json(json!({
    //     //             "error": "Email already in use"
    //     //         }));
    //     //     }
    //     //     Ok(None) => {}
    //     //     Err(err) => {
    //     //         let _ = transaction.rollback().await;
    //     //         tracing::error!("Database error {:#?}", err);
    //     //         return HttpResponse::BadRequest().json(json!({
    //     //             "error": "something went wrong"
    //     //         }));
    //     //     }
    //     // }

    //     let update_email_query = sqlx::query(
    //         r#"
	// 				UPDATE users SET email = $1, updated_at = $2 WHERE id = $3
	// 			"#,
    //     )
    //     .bind(new_email)
    //     .bind(Utc::now())
    //     .bind(user.id)
    //     .execute(&mut *transaction)
    //     .instrument(query_span.clone())
    //     .await;
    //     if let Err(err) = update_email_query {
    //         tracing::error!("Database error {:#?}", err);
    //         if let sqlx::Error::Database(db_err) = &err {
    //             if let Some(pg_err) = db_err.try_downcast_ref::<PgDatabaseError>() {
    //                 if pg_err.code() == "23505" {
    //                     // 23505 is the PostgreSQL error code for unique_violation
    //                     tracing::error!("Duplicate value error: {:#?}", err);
    //                     let _ = transaction.rollback().await;
    //                     return HttpResponse::Conflict().json(json!({
    //                         "error": "email is already taken",
    //                         "field": "email"
    //                     }));
    //                 }
    //             }
    //         }
    //         let _ = transaction.rollback().await;
    //         return HttpResponse::BadRequest().json(json!({
    //             "error": "something went wrong"
    //         }));
    // }
    // }

    if body.first_name.is_some() {
        tracing::info!("Updating user first name");
        let new_first_name = body.first_name.as_ref().unwrap();
        let update_first_name_query = sqlx::query(
            r#"
					UPDATE users SET first_name = $1, updated_at = $2 WHERE id = $3
				"#,
        )
        .bind(new_first_name)
        .bind(Utc::now())
        .bind(user.id)
        .execute(&mut *transaction)
        .instrument(query_span.clone())
        .await;
        if update_first_name_query.is_err() {
            tracing::error!("Database error {:#?}", update_first_name_query.unwrap_err());
            let _ = transaction.rollback().await;
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }

    if body.last_name.is_some() {
        tracing::info!("Updating user last_name");
        let new_last_name = body.last_name.as_ref().unwrap();
        let update_last_name_query = sqlx::query(
            r#"
					UPDATE users SET last_name = $1, updated_at = $2 WHERE id = $3
				"#,
        )
        .bind(new_last_name)
        .bind(Utc::now())
        .bind(user.id)
        .execute(&mut *transaction)
        .instrument(query_span.clone())
        .await;
        if update_last_name_query.is_err() {
            tracing::error!("Database error {:#?}", update_last_name_query.unwrap_err());
            let _ = transaction.rollback().await;
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }

    if body.username.is_some() {
        tracing::info!("Updating user username");
        let new_username = body.username.as_ref().unwrap();
        let update_username_query = sqlx::query(
            r#"
					UPDATE users SET username = $1, updated_at = $2, profile_is_finished = $3 WHERE id = $4
				"#,
        )
        .bind(new_username)
        .bind(Utc::now())
        .bind(true)
        .bind(user.id)
        .execute(&mut *transaction)
        .instrument(query_span.clone())
        .await;

        if let Err(err) = update_username_query {
            tracing::error!("Database error {:#?}", err);
            if let sqlx::Error::Database(db_err) = &err {
                if let Some(pg_err) = db_err.try_downcast_ref::<PgDatabaseError>() {
                    if pg_err.code() == "23505" {
                        // 23505 is the PostgreSQL error code for unique_violation
                        tracing::error!("Duplicate value error: {:#?}", err);
                        let _ = transaction.rollback().await;
                        return HttpResponse::Conflict().json(json!({
                            "error": "username is already taken",
                            "field": "username"
                        }));
                    }
                }
            }
            let _ = transaction.rollback().await;
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }

        // if update_username_query.is_err() {
        //     tracing::error!("Database error {:#?}", update_username_query.unwrap_err());
        //     let _ = transaction.rollback().await;
        //     return HttpResponse::BadRequest().json(json!({
        //         "error": "something went wrong"
        //     }));
        // }
    }

    match transaction.commit().await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("Failed to save transaction  {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }
    let new_user_info_res = sqlx::query!(
        r#"
            SELECT * FROM users WHERE id = $1
        "#,
        user.id
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await;

    match new_user_info_res {
        Ok(user) => {
            tracing::info!("Got user info");
            HttpResponse::Ok().json(json!({
                "data": {
                    "id": user.id.to_string(),
                    "email": user.email,
                    "first_name": user.first_name,
                    "last_name": user.last_name,
                    "created_at": user.created_at.to_string(),
                    "updated_at": user.updated_at.to_string(),
                    "username" : user.username,
                    "image_url": user.profile_picture_url
                }
            }))
        }
        Err(err) => {

            tracing::error!("Database error  {:#?}", err);
            HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }))
        }
    }
}
