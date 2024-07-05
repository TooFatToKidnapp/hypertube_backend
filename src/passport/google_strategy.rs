use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::GoogleStrategy;
use serde_json::json;
use sqlx::PgPool;
use std::env;
use tracing::Instrument;

use crate::routes::generate_token;
use chrono::Utc;
use super::AppState;

pub async fn google(passport: Data<AppState>) -> HttpResponse {
    tracing::info!("Google Oauth2 called");
    let mut auth = passport.google_passport.write().await;
    auth.authenticate("google");
    let url = auth.generate_redirect_url();
    HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, url))
        .finish()
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
                    tracing::info!(target: "query_span", "Got Google Profile");
                    profile
                },
                PassportResponse::FailureRedirect(failure) => {
                    tracing::info!("didn't get user profile. user redirected");
                    return HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, failure.to_string()))
                        .finish()
                }
            };
            res
        }
        Err(error) => {
            tracing::error!("Error: Bad Google Profile response");
            return HttpResponse::BadRequest().body(error.to_string())},
    };

    let user_email = &profile["emailAddresses"][0]["value"];
    if user_email.as_null() == Some(()) {
        tracing::error!("Error: user email not found in response");
        return HttpResponse::BadRequest().json(json!({
            "error": "Missing email from google payload"
        }));
    }

    let user_email = user_email.to_string().replace("\"", "");
    let query_result = sqlx::query!(
        r#"
            SELECT * FROM users WHERE email = $1
        "#,
        user_email
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span.clone())
    .await;

    match query_result {
        Ok(user) => {
            tracing::info!("Google Log in event. user email found in the database");
            let token_result = generate_token(user.id.to_string());
            if token_result.is_err() {
                tracing::error!("Failed to generate user token");
                return HttpResponse::InternalServerError().json(json!({
                    "error": "something went wrong"
                }));
            }
            println!("user email: {}", user.email);
            HttpResponse::Ok().json(json!({
                "data" : {
                    "token": token_result.unwrap(),
                    "email": user.email,
                    "created_at": user.created_at.to_string(),
                    "updated_at": user.updated_at.to_string(),
                    "username" : user.username,
                }
            }))
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("Google Sign up event. user email was not found in the database");
            let id = uuid::Uuid::new_v4();
            let user_name = &profile["names"][0]["givenName"];
            if user_name.as_null() == Some(()) {
                tracing::error!("Error: user name not found in response");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Missing name from google payload"
                }));
            }
            let user_name = user_name.to_string().replace("\"", "");
            let query_res = sqlx::query!(
                r#"
                    INSERT INTO users (id, username, email, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5)
                    RETURNING *
                "#,
                id,
                user_name,
                user_email,
                Utc::now(),
                Utc::now(),
            )
            .fetch_one(connection.get_ref())
            .instrument(query_span)
            .await;
            if query_res.is_err() {
                tracing::error!("Failed to create user {:?}", query_res.unwrap_err());
                return HttpResponse::InternalServerError().json(json!({
                    "error": "database error"
                }));
            }
            tracing::info!("Google Sign up event. user created successfully");
            let user = query_res.unwrap();
            let token_result = generate_token(user.id.to_string());
            if token_result.is_err() {
                tracing::error!("Failed to generate user token");
                return HttpResponse::InternalServerError().json(json!({
                    "error": "something went wrong"
                }));
            }
            HttpResponse::Ok().json(json!({
                "data" : {
                    "token": token_result.unwrap(),
                    "email": user.email,
                    "created_at": user.created_at.to_string(),
                    "updated_at": user.updated_at.to_string(),
                    "username" : user.username,
                }
            }))
        }
        Err(err) => {
            tracing::error!("database Error {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "something went wrong"
            }))
        }
    }

    // HttpResponse::BadRequest().finish()
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
