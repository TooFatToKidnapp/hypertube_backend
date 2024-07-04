use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use oauth2::{Scope, TokenUrl};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::Strategy;
use reqwest::Url;
use std::env;

use super::AppState;

#[derive(Clone, Debug)]
pub struct FortyTwoStrategy {
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub scopes: Vec<Scope>,
    pub request_uri: String,
    pub token_uri: String,
    pub redirect_uri: String,
    pub failure_redirect: String,
}

impl Default for FortyTwoStrategy {
    fn default() -> Self {
        FortyTwoStrategy {
            client_id: String::new(),
            client_secret: String::new(),
            token_uri: String::from("https://api.intra.42.fr/oauth/token"),
            auth_uri: String::from("https://api.intra.42.fr/oauth/authorize"),
            request_uri: String::from("https://api.intra.42.fr/v2/me"),
            scopes: Vec::new(),
            redirect_uri: String::new(),
            failure_redirect: String::new(),
        }
    }
}

impl FortyTwoStrategy {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
        failure_redirect: impl Into<String>,
        scopes: Vec<Scope>,
    ) -> Self {
        FortyTwoStrategy {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            failure_redirect: failure_redirect.into(),
            scopes,
            ..Default::default()
        }
    }
}

impl Strategy for FortyTwoStrategy {
    fn auth_url(&self) -> String {
        self.auth_uri.clone()
    }
    fn client_id(&self) -> String {
        self.client_id.clone()
    }
    fn client_secret(&self) -> String {
        self.client_secret.clone()
    }
    fn failure_redirect(&self) -> Url {
        match self.failure_redirect.parse::<Url>() {
            Ok(url) => url,
            Err(err) => panic!("{}{:?}", "Invalid Url", err),
        }
    }
    fn redirect_url(&self) -> String {
        self.redirect_uri.clone()
    }
    fn request_uri(&self) -> String {
        self.request_uri.clone()
    }
    fn scopes(&self) -> Vec<Scope> {
        self.scopes.clone()
    }
    fn token_url(&self) -> Option<TokenUrl> {
        match TokenUrl::new(self.token_uri.clone()) {
            Ok(token) => Some(token),
            Err(err) => panic!("{}{:?}", "Invalid Token Uri", err),
        }
    }
}

pub async fn forty_tow(passport: Data<AppState>) -> HttpResponse {
    let mut auth = passport.passport_42.write().await;
    auth.authenticate("42");
    let url = auth.generate_redirect_url();
    HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, url))
        .finish()
}

pub async fn authenticate_forty_two(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
) -> HttpResponse {
    let mut auth = auth.passport_42.write().await;
    // The `response` is an enum. It can either be a failure_redirect or profile
    match auth.get_profile(authstate.0).await {
        // The profile is a json value containing the user profile, access_token and refresh_token.
        Ok(response) => {
            let res = match response {
                // At this point you can proceed to save the profile info in the database or use the access token or refresh token to request for more user info or some other relevant info.
                PassportResponse::Profile(profile) => HttpResponse::Ok().json(profile),
                // If the user canceled the authorization process, a redirect to i.e login page would be very convinient rather
                // than displaying some `Internal server error` just to say. It may not be exactly that kind of error, but can be inclusive of others.
                PassportResponse::FailureRedirect(failure) => HttpResponse::SeeOther()
                    .append_header((http::header::LOCATION, failure.to_string()))
                    .finish(),
            };
            res
        }
        Err(error) => HttpResponse::BadRequest().body(error.to_string()),
    }
}

pub fn generate_forty_two_passport() -> PassPortBasicClient {
    let mut passport = PassPortBasicClient::default();
    let mut backend_url = env::var("BACKEND_URL").unwrap();
    backend_url.push_str("/redirect/42");

    passport.using(
        "42",
        FortyTwoStrategy::new(
            env::var("CLIENT_UID_42").unwrap(),
            env::var("CLIENT_SECRET_42").unwrap(),
            backend_url.as_str(),
            env::var("FAILURE_REDIRECT_URI").unwrap(),
            Vec::new(),
        ),
    );
    passport
}
