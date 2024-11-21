use actix_web::cookie::SameSite;
use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::GoogleStrategy;
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::PgPool;
use std::env;
use sqlx::Row;
use super::AppState;
use crate::middleware::User;
use crate::routes::create_session;
use chrono::Utc;
use tracing::Instrument;

pub async fn google(passport: Data<AppState>) -> HttpResponse {
    tracing::info!("Google Oauth2 called");
    let mut auth = passport.google_passport.write().await;
    auth.authenticate("google");
    let url = auth.generate_redirect_url();
    HttpResponse::Ok().json(json!({
        "redirect_url" : url
    }))
}

pub async fn authenticate_google(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Google Passport Event");

    let mut auth = auth.google_passport.write().await;
    auth.authenticate("google");
    let profile = match auth.get_profile(authstate.0).await {
        Ok(response) => {
            let res = match response {
                PassportResponse::Profile(profile) => {
                    tracing::info!("Got Google Profile");
                    profile
                }
                PassportResponse::FailureRedirect(failure) => {
                    tracing::info!("didn't get user Google profile. user redirected");
                    return HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, failure.to_string()))
                        .finish();
                }
            };
            res
        }
        Err(error) => {
            tracing::error!("Error: Bad Google Profile response");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };

    let user_email = &profile["emailAddresses"][0]["value"];
    if user_email.as_str().is_none() {
        tracing::error!("Error: user email not found in response");
        return HttpResponse::BadRequest().json(json!({
            "error": "Missing email from google payload"
        }));
    }

    let user_email = user_email.as_str().unwrap();
    let query_result = sqlx::query(
        r#"
            SELECT * FROM users WHERE email = $1
        "#,
    ).bind(user_email).map(|row: PgRow| {
        User {
            id: row.get("id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            image_url: row.get("profile_picture_url"),
            created_at: row.get::<chrono::NaiveDateTime, _>("created_at").to_string(),
            updated_at: row.get::<chrono::NaiveDateTime, _>("updated_at").to_string(),
            username: row.get("username"),
            session_id: None,
        }
    })
    .fetch_one(connection.get_ref())
    .instrument(query_span.clone())
    .await;

    match query_result {
        Ok(user) => {
            tracing::info!("Google Log in event. user email found in the database");
            // let user = User {
            //     id: user.id,
            //     first_name: user.first_name,
            //     last_name: user.last_name,
            //     image_url: user.profile_picture_url,
            //     email: user.email,
            //     created_at: user.created_at.to_string(),
            //     updated_at: user.updated_at.to_string(),
            //     username: user.username,
            //     session_id: None,
            // };
            let session_result =
                create_session(connection.as_ref(), user.clone(), SameSite::None).await;
            if session_result.is_err() {
                tracing::error!(
                    "Failed to generate user session  {}",
                    session_result.unwrap_err()
                );
                return HttpResponse::InternalServerError().json(json!({
                    "error": "something went wrong"
                }));
            }
            HttpResponse::Ok().cookie(session_result.unwrap()).finish()
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("Google Sign up event. user email was not found in the database");
            let id = uuid::Uuid::new_v4();
            let user_name = &profile["names"][0]["givenName"];
            if user_name.as_str().is_none() {
                tracing::error!("Error: user name not found in response");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Missing name from google payload"
                }));
            }
            let user_name = user_name.as_str().unwrap();
            let query_res = sqlx::query(
                r#"
                    INSERT INTO users (id, username, email, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5)
                    RETURNING *
                "#,
                // uuid::Uuid::new_v4(),
                // user_name,
                // // image_url,
                // email,
                // Utc::now(),
                // Utc::now(),
            ).bind(uuid::Uuid::new_v4()).bind(user_name).bind(user_email).bind(Utc::now()).bind(Utc::now())
            .map(|row: PgRow|{
                User {
                    id: row.get("id"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    email: row.get("email"),
                    image_url: row.get("profile_picture_url"),
                    created_at: row.get::<chrono::NaiveDateTime, _>("created_at").to_string(),
                    updated_at: row.get::<chrono::NaiveDateTime, _>("updated_at").to_string(),
                    username: row.get("username"),
                    session_id: None,
                }
            })
            .fetch_one(connection.get_ref())
            .instrument(query_span).await;

            if query_res.is_err() {
                tracing::error!("Failed to create user {:?}", query_res.unwrap_err());
                return HttpResponse::InternalServerError().json(json!({
                    "error": "database error"
                }));
            }
            tracing::info!("Google Sign up event. user created successfully");
            let user = query_res.unwrap();
            // let user = User {
            //     id: user_rec.id,
            //     first_name: user_rec.first_name,
            //     last_name: user_rec.last_name,
            //     image_url: user_rec.profile_picture_url,
            //     email: user_rec.email,
            //     created_at: user_rec.created_at.to_string(),
            //     updated_at: user_rec.updated_at.to_string(),
            //     username: user_rec.username,
            //     session_id: None,
            // };
            let session_result =
                create_session(connection.as_ref(), user.clone(), SameSite::None).await;
            if session_result.is_err() {
                tracing::error!(
                    "Failed to generate user session  {}",
                    session_result.unwrap_err()
                );
                return HttpResponse::InternalServerError().json(json!({
                    "error": "something went wrong"
                }));
            }
            HttpResponse::Ok().cookie(session_result.unwrap()).finish()
        }
        Err(err) => {
            tracing::error!("database Error {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "something went wrong"
            }))
        }
    }
}

pub fn generate_google_passport() -> PassPortBasicClient {
    let mut passport = PassPortBasicClient::default();
    let scope = env::var("GOOGLE_CLIENT_SCOPE").unwrap();
    let scopes: Vec<&str> = scope.split(',').collect();
    let mut backend_url = env::var("BACKEND_URL").unwrap();
    backend_url.push_str("/redirect/google");

    passport.using(
        "google",
        GoogleStrategy::new(
            env::var("GOOGLE_CLIENT_ID").unwrap().as_str(),
            env::var("GOOGLE_CLIENT_SECRET").unwrap().as_str(),
            scopes,
            backend_url.as_str(),
            env::var("FAILURE_REDIRECT_URI").unwrap().as_str(),
        ),
    );
    passport
}
