use actix_web::web::Query;
use actix_web::HttpResponse;
use actix_web::{
    cookie::SameSite,
    http,
    web::Data,
};
use passport_strategies::passport::{Choice, Passport, StateCode};

use sqlx::PgPool;
use tokio::sync::RwLock;


use crate::middleware::User;
use crate::routes::create_session;

use serde_json::json;

// #[derive(Debug, Clone)]
// pub struct FortyTwoStrategy {
//     pub client_id: String,
//     pub client_secret: String,
//     pub auth_uri: String,
//     pub scopes: Vec<Scope>,
//     pub request_uri: String,
//     pub token_uri: String,
//     pub redirect_uri: String,
//     pub failure_redirect: String,
// }

// impl Default for FortyTwoStrategy {
//     fn default() -> Self {
//         FortyTwoStrategy {
//             client_id: String::new(),
//             client_secret: String::new(),
//             token_uri: String::from("https://api.intra.42.fr/oauth/token"),
//             auth_uri: String::from("https://api.intra.42.fr/oauth/authorize"),
//             request_uri: String::from("https://api.intra.42.fr/v2/me"),
//             scopes: Vec::new(),
//             redirect_uri: String::new(),
//             failure_redirect: String::new(),
//         }
//     }
// }

// impl FortyTwoStrategy {
//     pub fn new(
//         client_id: impl Into<String>,
//         client_secret: impl Into<String>,
//         redirect_uri: impl Into<String>,
//         failure_redirect: impl Into<String>,
//         scopes: Vec<Scope>,
//     ) -> Self {
//         FortyTwoStrategy {
//             client_id: client_id.into(),
//             client_secret: client_secret.into(),
//             redirect_uri: redirect_uri.into(),
//             failure_redirect: failure_redirect.into(),
//             scopes,
//             ..Default::default()
//         }
//     }
// }

// impl Strategy for FortyTwoStrategy {
//     fn auth_url(&self) -> String {
//         self.auth_uri.clone()
//     }
//     fn client_id(&self) -> String {
//         self.client_id.clone()
//     }
//     fn client_secret(&self) -> String {
//         self.client_secret.clone()
//     }
//     fn failure_redirect(&self) -> Url {
//         match self.failure_redirect.parse::<Url>() {
//             Ok(url) => url,
//             Err(err) => panic!("{}{:?}", "Invalid Url", err),
//         }
//     }
//     fn redirect_url(&self) -> String {
//         self.redirect_uri.clone()
//     }
//     fn request_uri(&self) -> String {
//         self.request_uri.clone()
//     }
//     fn scopes(&self) -> Vec<Scope> {
//         self.scopes.clone()
//     }
//     fn token_url(&self) -> Option<TokenUrl> {
//         match TokenUrl::new(self.token_uri.clone()) {
//             Ok(token) => Some(token),
//             Err(err) => panic!("{}{:?}", "Invalid Token Uri", err),
//         }
//     }
// }

// pub async fn forty_tow(passport: Data<AppState>) -> HttpResponse {
//     let mut auth = passport.passport_42.write().await;
//     auth.authenticate("42");
//     let url = auth.generate_redirect_url();
//     HttpResponse::Ok().json(json!({
//         "redirect_url" : url
//     }))
// }

pub async fn authenticate_forty_two(
    passport: Data<RwLock<Passport>>,
    Query(statecode): Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("42 Passport Event");

    let mut auth = passport.write().await;
    let (profile, success_redirect_url) = match auth.authenticate(Choice::FortyTwo, statecode).await {
        (Some(response), url) => {
            tracing::info!("Got 42 Profile");
            (response.profile, url)
        }
        (None, url) => {
            tracing::info!("didn't get user 42 profile. user redirected");
            return HttpResponse::SeeOther()
                .append_header((http::header::LOCATION, url))
                .finish();
        }
    };

    if profile["first_name"].as_str().is_none() {
        tracing::error!("didn't find a valid first_name in 42 payload");
        return HttpResponse::BadRequest().json(json!({
            "error": "didn't find a valid first_name in 42 payload"
        }));
    }
    let first_name = profile["first_name"].as_str().unwrap();

    if profile["last_name"].as_str().is_none() {
        tracing::error!("didn't find a valid last_name in 42 payload");
        return HttpResponse::BadRequest().json(json!({
            "error": "didn't find a valid last_name in 42 payload"
        }));
    }
    let last_name = profile["last_name"].as_str().unwrap();

    if profile["email"].as_str().is_none() {
        tracing::error!("didn't find a valid email in 42 payload");
        return HttpResponse::BadRequest().json(json!({
            "error": "didn't find a valid email in 42 payload"
        }));
    }
    let user_email = profile["email"].as_str().unwrap();

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
            tracing::info!("42 Log in event. user email found in the database");
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
            tracing::info!("42 Sign up event. user email was not found in the database");
            let id = uuid::Uuid::new_v4();
            let user_name = &profile["login"];
            if user_name.as_str().is_none() {
                tracing::error!("Error: user name not found in response");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Missing name from 42 payload"
                }));
            }
            let user_name = user_name.as_str().unwrap();
            let query_res = sqlx::query!(
                r#"
                    INSERT INTO users (id, username, email, first_name, last_name, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    RETURNING *
                "#,
                id,
                user_name,
                user_email,
                first_name,
                last_name,
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
            let user_res = query_res.unwrap();
            tracing::info!("42 Sign up event. user created successfully");
            let user = User {
                id: user_res.id,
                first_name: user_res.first_name,
                last_name: user_res.last_name,
                image_url: user_res.profile_picture_url,
                email: user_res.email,
                created_at: user_res.created_at.to_string(),
                updated_at: user_res.updated_at.to_string(),
                username: user_res.username,
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

// pub fn generate_forty_two_passport() -> PassPortBasicClient {
//     let mut passport = PassPortBasicClient::default();
//     let mut backend_url = env::var("BACKEND_URL").unwrap();
//     backend_url.push_str("/redirect/42");
//     passport.using(
//         "42",
//         FortyTwoStrategy::new(
//             env::var("CLIENT_UID_42").unwrap(),
//             env::var("CLIENT_SECRET_42").unwrap(),
//             backend_url.as_str(),
//             env::var("FAILURE_REDIRECT_URI").unwrap(),
//             Vec::new(),
//         ),
//     );
//     passport
// }
