use actix_web::{
    web::{self, Data},
    Scope,
};
use passport_strategies::basic_client::PassPortBasicClient;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    authenticate_forty_two, authenticate_github, authenticate_google, forty_tow,
    generate_forty_two_passport, generate_github_passport, generate_google_passport, github,
    google,
};

fn passport_route_redirect() -> Scope {
    web::scope("/redirect")
        .route("/google", web::get().to(authenticate_google))
        .route("/42", web::get().to(authenticate_forty_two))
        .route("/github", web::get().to(authenticate_github))
}

fn passport_route_auth() -> Scope {
    web::scope("/auth")
        .route("/google", web::get().to(google))
        .route("/42", web::get().to(forty_tow))
        .route("/github", web::get().to(github))
}

#[derive(Clone)]
pub struct AppState {
    pub google_passport: Arc<RwLock<PassPortBasicClient>>,
    pub passport_42: Arc<RwLock<PassPortBasicClient>>,
    pub github_passport: Arc<RwLock<PassPortBasicClient>>,
}

pub fn configure_passport_service(cfg: &mut web::ServiceConfig) {
    let google_passport = generate_google_passport();
    let google_passport_clone: Arc<RwLock<PassPortBasicClient>> =
        Arc::new(RwLock::new(google_passport));

    let passport_42 = generate_forty_two_passport();
    let passport_42_clone: Arc<RwLock<PassPortBasicClient>> = Arc::new(RwLock::new(passport_42));

    let github_passport = generate_github_passport();
    let github_passport_clone = Arc::new(RwLock::new(github_passport));

    let state = AppState {
        google_passport: google_passport_clone.clone(),
        github_passport: github_passport_clone.clone(),
        passport_42: passport_42_clone.clone(),
    };

    // cfg.app_data(Data::new(google_passport_clone.clone()))
    //     .app_data(Data::new(github_passport_clone.clone()))
    //     .app_data(Data::new(passport_42_clone.clone()))
    cfg.app_data(Data::new(state.clone()))
        .service(passport_route_auth())
        .service(passport_route_redirect());
}
