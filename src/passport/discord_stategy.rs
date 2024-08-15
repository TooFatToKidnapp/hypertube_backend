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

pub async fn authenticate_discord(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Github Passport Event");
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
    println!("{}", profile);
    todo!()
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
