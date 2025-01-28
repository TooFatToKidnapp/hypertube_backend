use actix_web::cookie::SameSite;
use actix_web::web::Query;
use actix_web::HttpResponse;
use actix_web::{http, web::Data};
use passport_strategies::passport::{Choice, Passport, StateCode};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::middleware::User;
use crate::routes::create_session;
use sqlx::types::chrono::Utc;
use tracing::Instrument;


pub async fn google(passport: Data<RwLock<Passport>>) -> HttpResponse {
    tracing::info!("CALLING GOOGLE OAUTH");
    let mut auth = passport.write().await;

    let url = auth.redirect_url(Choice::Google);

    tracing::info!("{:#?}", url);
    HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, url))
        .finish()
}

pub async fn authenticate_google(
    passport: Data<RwLock<Passport>>,
    Query(statecode): Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Google Passport Event");

    let mut auth = passport.write().await;
    let (profile, success_redirect_url) = match auth.authenticate(Choice::Google, statecode).await {
        (Some(response), url) => {
            tracing::info!("Got Google Profile");
            (response.profile, url)
        }
        (None, url) => {
            tracing::info!("didn't get user Google profile. user redirected");
            return HttpResponse::SeeOther()
                .append_header((http::header::LOCATION, url))
                .finish();
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
            tracing::info!("Google Sign up event. user created successfully");
            let user_rec = query_res.unwrap();
            let user = User {
                id: user_rec.id,
                first_name: user_rec.first_name,
                last_name: user_rec.last_name,
                image_url: user_rec.profile_picture_url,
                email: user_rec.email,
                created_at: user_rec.created_at.to_string(),
                updated_at: user_rec.updated_at.to_string(),
                username: user_rec.username,
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
            HttpResponse::Ok()
                .cookie(session_result.unwrap())
                .json(json!( {
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

// pub fn generate_google_passport() -> PassPortBasicClient {
//     let mut passport = PassPortBasicClient::default();
//     let scope = env::var("GOOGLE_CLIENT_SCOPE").unwrap();
//     let scopes: Vec<&str> = scope.split(',').collect();
//     let mut backend_url = env::var("BACKEND_URL").unwrap();
//     backend_url.push_str("/redirect/google");

//     passport.using(
//         "google",
//         GoogleStrategy::new(
//             env::var("GOOGLE_CLIENT_ID").unwrap().as_str(),
//             env::var("GOOGLE_CLIENT_SECRET").unwrap().as_str(),
//             scopes,
//             backend_url.as_str(),
//             env::var("FAILURE_REDIRECT_URI").unwrap().as_str(),
//         ),
//     );
//     passport
// }
