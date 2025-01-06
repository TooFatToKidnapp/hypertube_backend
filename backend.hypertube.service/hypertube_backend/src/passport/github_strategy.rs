use actix_web::cookie::SameSite;
use actix_web::web::Query;
use actix_web::HttpResponse;
use actix_web::{
    http,
    web::Data,
};
use passport_strategies::passport::{Choice, Passport, StateCode};
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::middleware::User;
use crate::routes::create_session;

use serde_json::json;

// pub async fn github(passport: Data<AppState>) -> HttpResponse {
//     let mut auth = passport.github_passport.write().await;
//     auth.authenticate("github");
//     let url = auth.generate_redirect_url();
//     HttpResponse::Ok().json(json!({
//         "redirect_url" : url
//     }))
// }

pub async fn authenticate_github(
    passport: Data<RwLock<Passport>>,
    Query(statecode): Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Github Passport Event");

    let mut auth = passport.write().await;
    let (profile, access_token,  success_redirect_url) = match auth.authenticate(Choice::Github, statecode).await {
        (Some(response), url) => {
            tracing::info!("Got Github Profile");
            (response.profile, response.access_token.0 , url)
        }
        (None, url) => {
            tracing::info!("didn't get user Github profile. user redirected");
            return HttpResponse::SeeOther()
                .append_header((http::header::LOCATION, url))
                .finish();
        }
    };

    // if profile["access_token"].as_str().is_none() {
    //     tracing::error!("Didn't find access token in the response");
    //     return HttpResponse::BadRequest().json(json!({
    //         "error": "Missing access token in github response"
    //     }));
    // }
    // let access_token = profile["access_token"].as_str().unwrap();

    let client = reqwest::Client::new();
    let request = client
        .get("https://api.github.com/user/emails")
        .header(http::header::ACCEPT, "application/vnd.github+json")
        .header(
            http::header::AUTHORIZATION,
            format!("Bearer {}", access_token),
        )
        .header(http::header::USER_AGENT, "HyperTube");

    let response = request.send().await;
    if response.is_err() {
        tracing::error!("couldn't send request to github api");
        return HttpResponse::BadRequest().json(json!({
            "error": "couldn't send request to github api"
        }));
    }
    let body = response.unwrap().text().await;
    if body.is_err() {
        tracing::error!("couldn't get response body");
        return HttpResponse::BadRequest().json(json!({
            "error": "couldn't get response body"
        }));
    }
    let parsing_result = serde_json::from_str::<serde_json::Value>(body.unwrap().as_str());
    if parsing_result.as_ref().is_err() {
        tracing::error!("Bad response body from the github api");
        return HttpResponse::BadRequest().json(json!({
            "error": "Bad response body from the github api"
        }));
    }
    if parsing_result.as_ref().unwrap().as_array().is_none() {
        tracing::error!("Bad response body from the github api");
        return HttpResponse::BadRequest().json(json!({
            "error": "Bad response body from the github api"
        }));
    }
    let mut user_email = String::new();
    for elem in parsing_result.as_ref().unwrap().as_array().unwrap().iter() {
        if elem["primary"].is_boolean() && elem["primary"].as_bool() == Some(true) {
            let email_res = elem["email"].as_str();
            if let Some(email) = email_res {
                email.clone_into(&mut user_email);
                break;
            }
        }
    }
    if user_email.is_empty() {
        tracing::error!("Can't get user email from the github api");
        return HttpResponse::BadRequest().json(json!({
            "error": "Can't get user email from the github api"
        }));
    }

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
            let session_result =
                create_session(connection.as_ref(), user.clone(), SameSite::None).await;
            if session_result.is_err() {
                tracing::error!(
                    "Failed to generate user session  {}",
                    session_result.unwrap_err()
                );
                return HttpResponse::BadRequest().json(json!({
                    "error": "something went wrong"
                }));
            }
            HttpResponse::Ok().cookie(session_result.unwrap()).finish()
        }
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("Github Sign up event. user email was not found in the database");
            let id = uuid::Uuid::new_v4();
            let user_name = &profile["login"];
            if user_name.as_str().is_none() {
                tracing::error!("Error: user name not found in response");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Missing name from Github payload"
                }));
            }
            let user_name = user_name.as_str().unwrap();
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
                return HttpResponse::BadRequest().json(json!({
                    "error": "database error"
                }));
            }
            tracing::info!("Github Sign up event. user created successfully");
            let user = query_res.unwrap();
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
            let session_result =
                create_session(connection.as_ref(), user.clone(), SameSite::None).await;
            if session_result.is_err() {
                tracing::error!(
                    "Failed to generate user session  {}",
                    session_result.unwrap_err()
                );
                return HttpResponse::BadRequest().json(json!({
                    "error": "something went wrong"
                }));
            }
            HttpResponse::Ok().cookie(session_result.unwrap()).json(json!({
                "url" : success_redirect_url
            }))
        }
        Err(err) => {
            tracing::error!("database Error {:#?}", err);
            HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }))
        }
    }
}

// pub fn generate_github_passport() -> PassPortBasicClient {
//     let mut passport = PassPortBasicClient::default();
//     let scope = env::var("CLIENT_SCOPE_GITHUB").unwrap();
//     let scopes: Vec<&str> = scope.split(',').collect();
//     let mut backend_url = env::var("BACKEND_URL").unwrap();
//     backend_url.push_str("/redirect/github");
//     passport.using(
//         "github",
//         GithubStrategy::new(
//             env::var("CLIENT_ID_GITHUB").unwrap().as_str(),
//             env::var("CLIENT_SECRET_GITHUB").unwrap().as_str(),
//             scopes,
//             backend_url.as_str(),
//             env::var("FAILURE_REDIRECT_URI").unwrap().as_str(),
//         ),
//     );
//     passport
// }
