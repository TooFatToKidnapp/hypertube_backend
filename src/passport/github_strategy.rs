use actix_web::http::header;
use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::GithubStrategy;
use sqlx::PgPool;
use std::env;

use super::AppState;
use crate::routes::generate_token;
use tracing::Instrument;
use chrono::Utc;
use serde_json::json;

pub async fn github(passport: Data<AppState>) -> HttpResponse {
    let mut auth = passport.github_passport.write().await;
    auth.authenticate("github");
    let url = auth.generate_redirect_url();
    HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, url))
        .finish()
}
// #[derive()]
struct ErrorType {
    error: String,
    error_description: String,
    error_uri: String
}

pub async fn authenticate_github(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Github Passport Event");

    let mut auth = auth.github_passport.write().await;
    auth.authenticate("github");
    let profile = match auth.get_profile(authstate.0).await {
        Ok(response) => {
            let res = match response {
                PassportResponse::Profile(profile) => {
                    tracing::info!("Got Github Profile");
                    profile
                }
                PassportResponse::FailureRedirect(failure) => {
                    tracing::info!("didn't get user Github profile. user redirected");
                    return HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, failure.to_string()))
                        .finish();
                }
            };
            res
        }
        Err(error) => {
            tracing::error!("Error: Bad Github Profile response");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };

    // return HttpResponse::Ok().json(profile);

    if profile["access_token"].is_null() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Missing access token"
        }));
    }
    let access_token = profile["access_token"].as_str().unwrap();
    println!("access_token [{}]", access_token);
    let client = reqwest::Client::new();
    let request = client.get(
        "https://api.github.com/user/emails"
    ).header(http::header::ACCEPT, "application/vnd.github+json")
    .header(http::header::AUTHORIZATION, format!("Bearer {}", access_token))
    .header(http::header::USER_AGENT, "HyperTube");
    // .header("X-GitHub-Api-Version", "2022-11-28");
println!("REQUEST : {:#?}", request);
    let email_response = request.send().await.expect("Error sending response").text().await.unwrap();
    println!("email_response {:#?}", email_response);
    let parsing_result = serde_json::from_str::<serde_json::Value>(&email_response);
    let mut user_email = String::new();
    for elem in parsing_result.as_ref().unwrap().as_array().unwrap().iter() {
        println!("elem : {}", elem);
        if elem["primary"].is_boolean() && elem["primary"].as_bool() == Some(true) {
            user_email = elem["email"].as_str().unwrap().to_string();
        }
    }
    println!("USER EMAIL {}", user_email);
    match parsing_result {
        Ok(data) => {
            return HttpResponse::Ok().json(json!({
                "res": data
            }));
        },
        Err(err) => {
            println!("err : {:#?}", err);
            return HttpResponse::InternalServerError().json(json!({
                "err": "shit broke"
            }));
        }
    }

    let user_email = &profile["emailAddresses"][0]["value"];
    if user_email.is_null() {
        tracing::error!("Error: user email not found in response");
        return HttpResponse::BadRequest().json(json!({
            "error": "Missing email from Github payload"
        }));
    }

    let user_email = user_email.to_string().replace('"', "");
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
            tracing::info!("Github Log in event. user email found in the database");
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
            tracing::info!("Github Sign up event. user email was not found in the database");
            let id = uuid::Uuid::new_v4();
            let user_name = &profile["login"];
            if user_name.is_null() {
                tracing::error!("Error: user name not found in response");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Missing name from Github payload"
                }));
            }
            let user_name = user_name.to_string().replace('"', "");
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
            tracing::info!("Github Sign up event. user created successfully");
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
}

// curl -L \
//   -H "Accept: application/vnd.github+json" \
//   -H "Authorization: Bearer <the access_token from the user>" \
//   -H "X-GitHub-Api-Version: 2022-11-28" \
//   https://api.github.com/user/emails

pub fn generate_github_passport() -> PassPortBasicClient {
    let mut passport = PassPortBasicClient::default();
    let scope = env::var("GITHUB_CLIENT_SCOPE").unwrap();
    let scopes: Vec<&str> = scope.split(',').collect();
    let mut backend_url = env::var("BACKEND_URL").unwrap();
    backend_url.push_str("/redirect/github");
    passport.using(
        "github",
        GithubStrategy::new(
            env::var("GITHUB_CLIENT_ID").unwrap().as_str(),
            env::var("GITHUB_CLIENT_SECRET").unwrap().as_str(),
            scopes,
            backend_url.as_str(),
            env::var("FAILURE_REDIRECT_URI").unwrap().as_str(),
        ),
    );
    passport
}
