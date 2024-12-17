use actix_web::cookie::SameSite;
use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::DiscordStrategy;
use sqlx::PgPool;
use std::env;

use crate::middleware::User;
use crate::routes::create_session;

use super::AppState;
use chrono::Utc;
use serde_json::json;
use tracing::Instrument;

pub async fn discord(passport: Data<AppState>) -> HttpResponse {
    let mut auth = passport.discord_passport.write().await;
    auth.authenticate("discord");
    let url = auth.generate_redirect_url();
    HttpResponse::Ok().json(json!({
        "redirect_url" : url
    }))
}

// add user exist

async fn create_user(
    connection: &PgPool,
    profile: &serde_json::Value,
    query_span: tracing::span::Span,
) -> HttpResponse {
    let user_name = &profile["username"];
    // let image_url = &profile["avatar"];

    let user_name = user_name.as_str();
    // let image_url = image_url.as_str();

    if user_name.is_none() {
        return HttpResponse::BadRequest().json(json!(
            {
                "error":"missing some informations from discord response: username",
            }
        ));
    }

    let email = profile["email"].as_str().unwrap();
    let query_result = sqlx::query!(
        r#"
            INSERT INTO users (id, username, email, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
        "#,
        uuid::Uuid::new_v4(),
        user_name,
        email,
        Utc::now(),
        Utc::now(),
    )
    .fetch_one(connection)
    .instrument(query_span)
    .await;

    match query_result {
        Ok(user) => {
            let new_user = User {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
                image_url: user.profile_picture_url,
                email: user.email,
                created_at: user.created_at.to_string(),
                updated_at: user.updated_at.to_string(),
                username: user.username,
                session_id: None,
            };
            let session = create_session(connection, new_user.clone(), SameSite::None).await;
            match session {
                Ok(cookie) => {
                    return HttpResponse::Ok()
                        .cookie(cookie)
                        .json(json!({"OK":"user was created"}));
                }
                Err(_) => HttpResponse::InternalServerError().json(json!({
                    "error":"failed to generate session for user",
                })),
            }
        }
        Err(_) => HttpResponse::InternalServerError().json(json!({
            "error":"failed to create user",
        })),
    }
}

pub async fn authenticate_discord(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Discord Passport Event");
    println!(
        "got query {:#?}",
        json!({
            "data" : authstate.0
        })
    );

    let mut auth = auth.discord_passport.write().await;

    auth.authenticate("discord");
    let profile = match auth.get_profile(authstate.0).await {
        Ok(response) => {
            let res = match response {
                PassportResponse::Profile(profile) => {
                    tracing::info!("Got Discord Profile");
                    profile
                }
                PassportResponse::FailureRedirect(failure) => {
                    println!("------- FAILURE redirect from no PROFILE ---------");
                    tracing::info!("didn't get user Discord profile. user redirected");
                    return HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, failure.to_string()))
                        .finish();
                }
            };
            res
        }
        Err(error) => {
            tracing::error!("Error: Bad Discord Profile response");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };

    let email = &profile["email"];
    let is_verified = &profile["verified"];

    if email.as_str().is_none() || is_verified == false {
        if is_verified == false {
            return HttpResponse::BadRequest().body("email is not verified by discord");
        }
        return HttpResponse::BadRequest().body("No email provided by discord");
    }

    let email = email.as_str().unwrap();
    let query_result = sqlx::query!(
        r#"
            SELECT * FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_one(connection.get_ref())
    .instrument(query_span.clone())
    .await;

    match query_result {
        Ok(user) => {
            let user = User {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
                image_url: user.profile_picture_url,
                email: user.email,
                created_at: user.created_at.to_string(),
                updated_at: user.updated_at.to_string(),
                username: user.username,
                session_id: None,
            };

            tracing::info!("Google Log in event. user email found in the database");
            let session_result =
                create_session(connection.as_ref(), user.clone(), SameSite::None).await;
            match session_result {
                Ok(cookie) => {
                    return HttpResponse::Ok()
                        .cookie(cookie)
                        .json(json!({"OK":"user was found"}));
                }
                Err(_) => HttpResponse::InternalServerError().json(json!({
                    "Error":"failed to generate user session",
                })),
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            create_user(connection.get_ref(), &profile, query_span.clone()).await
        }
        Err(err) => {
            tracing::error!("database Error {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error" : "something went wrong"
            }))
        }
    }
}

pub fn generate_discord_passport() -> PassPortBasicClient {
    let mut passport = PassPortBasicClient::default();
    let mut backend_url = env::var("BACKEND_URL").unwrap();

    let scope = env::var("CLIENT_SCOPE_DISCORD").unwrap();
    let scopes: Vec<&str> = scope.split(',').collect();

    backend_url.push_str("/redirect/discord");
    passport.using(
        "discord",
        DiscordStrategy::new(
            env::var("CLIENT_ID_DISCORD").unwrap().as_str(),
            env::var("CLIENT_SECRET_DISCORD").unwrap().as_str(),
            scopes,
            backend_url.as_str(),
            env::var("FAILURE_REDIRECT_URI").unwrap().as_str(),
        ),
    );
    passport
}