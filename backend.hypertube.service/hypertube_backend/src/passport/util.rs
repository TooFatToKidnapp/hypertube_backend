use actix_web::{web, Scope};
// use passport_strategies::basic_client::PassPortBasicClient;
use passport_strategies::{
    passport::{Choice, Passport, Redirect},
    strategies::DiscordStrategy,
};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    authenticate_discord, authenticate_forty_two, authenticate_github, authenticate_google,
    discord, forty_tow, github, google,
};

pub fn passport_route_redirect() -> Scope {
    web::scope("/redirect")
        .route("/google", web::get().to(authenticate_google))
        .route("/42", web::get().to(authenticate_forty_two))
        .route("/github", web::get().to(authenticate_github))
        .route("/discord", web::get().to(authenticate_discord))
}

pub fn passport_route_auth() -> Scope {
    web::scope("/auth")
        .route("/google", web::get().to(google))
        .route("/42", web::get().to(forty_tow))
        .route("/github", web::get().to(github))
        .route("/discord", web::get().to(discord))
}

// #[derive(Clone)]
// pub struct AppState {
//     pub google_passport: Arc<RwLock<PassPortBasicClient>>,
//     pub passport_42: Arc<RwLock<PassPortBasicClient>>,
//     pub github_passport: Arc<RwLock<PassPortBasicClient>>,
//     pub discord_passport: Arc<RwLock<PassPortBasicClient>>,
// }

pub fn generate_passports() -> Result<Passport, Box<dyn std::error::Error>> {
    let passport = Passport::default()
        .redirect_urls(Redirect::new("", "")?)
        .strategize(
            Choice::Discord,
            DiscordStrategy::new(
                env::var("CLIENT_ID_DISCORD").unwrap().as_str(),
                env::var("CLIENT_SECRET_DISCORD").unwrap().as_str(),
                &["email", "identify"],
                "http://127.0.0.1:8000/redirect/discrod",
            ),
        )?
        .strategize(
            Choice::FortyTwo,
            DiscordStrategy::new(
                env::var("CLIENT_UID_42").unwrap().as_str(),
                env::var("CLIENT_SECRET_42").unwrap().as_str(),
                &[],
                "http://127.0.0.1:8000/redirect/42",
            ),
        )?
        .strategize(
            Choice::Google,
            DiscordStrategy::new(
                env::var("GOOGLE_CLIENT_ID").unwrap().as_str(),
                env::var("GOOGLE_CLIENT_SECRET").unwrap().as_str(),
                &[
                    "https://www.googleapis.com/auth/userinfo.email",
                    "https://www.googleapis.com/auth/userinfo.profile",
                ],
                "http://127.0.0.1:8000/redirect/google",
            ),
        )?
        .strategize(
            Choice::Github,
            DiscordStrategy::new(
                env::var("CLIENT_ID_GITHUB").unwrap().as_str(),
                env::var("CLIENT_SECRET_GITHUB").unwrap().as_str(),
                &["user:email"],
                "http://127.0.0.1:8000/redirect/github",
            ),
        )?;

    Ok(passport)
}

// pub fn configure_passport_service() -> AppState {
//     let google_passport = generate_google_passport();
//     let google_passport_clone = Arc::new(RwLock::new(google_passport));

//     let passport_42 = generate_forty_two_passport();
//     let passport_42_clone = Arc::new(RwLock::new(passport_42));

//     let github_passport = generate_github_passport();
//     let github_passport_clone = Arc::new(RwLock::new(github_passport));

//     let discord_passport = generate_discord_passport();
//     let discord_passport_clone = Arc::new(RwLock::new(discord_passport));

//     AppState {
//         google_passport: google_passport_clone,
//         github_passport: github_passport_clone,
//         passport_42: passport_42_clone,
//         discord_passport: discord_passport_clone,
//     }
// }
