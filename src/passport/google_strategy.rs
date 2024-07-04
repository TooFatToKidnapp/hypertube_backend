use actix_web::HttpResponse;
use actix_web::{
    http,
    web::{self, Data},
};
use passport_strategies::basic_client::{PassPortBasicClient, PassportResponse, StateCode};
use passport_strategies::strategies::GoogleStrategy;
use std::env;

use super::AppState;

pub async fn google(passport: Data<AppState>) -> HttpResponse {
    let mut auth = passport.google_passport.write().await;
    auth.authenticate("google");
    let url = auth.generate_redirect_url();
    HttpResponse::SeeOther()
        .append_header((http::header::LOCATION, url))
        .finish()
}

// pub async fn authenticate_google(
// 	auth: Data<Arc<RwLock<PassPortBasicClient>>>,
// 	authstate: web::Query<StateCode>,
// ) -> HttpResponse {
// 	let mut auth = auth.write().await;
// 	// The `response` is an enum. It can either be a failure_redirect or profile
// 	match auth.get_profile(authstate.0).await {
// 			// The profile is a json value containing the user profile, access_token and refresh_token.
// 			Ok(response) => {
// 					let res = match response {
// 							// At this point you can proceed to save the profile info in the database or use the access token or refresh token to request for more user info or some other relevant info.
// 							PassportResponse::Profile(profile) => HttpResponse::Ok().json(profile),
// 							// If the user canceled the authorization process, a redirect to i.e login page would be very convinient rather
// 							// than displaying some `Internal server error` just to say. It may not be exactly that kind of error, but can be inclusive of others.
// 							PassportResponse::FailureRedirect(failure) => HttpResponse::SeeOther()
// 									.append_header((http::header::LOCATION, failure.to_string()))
// 									.finish(),
// 					};
// 					res
// 			}
// 			Err(error) => HttpResponse::BadRequest().body(error.to_string()),
// 	}
// }

pub async fn authenticate_google(
    auth: Data<AppState>,
    authstate: web::Query<StateCode>,
) -> HttpResponse {
    let mut auth = auth.google_passport.write().await;
    // The `response` is an enum. It can either be a failure_redirect or profile
    let profile = match auth.get_profile(authstate.0).await {
        // The profile is a json value containing the user profile, access_token and refresh_token.
        Ok(response) => {
            let res = match response {
                // At this point you can proceed to save the profile info in the database or use the access token or refresh token to request for more user info or some other relevant info.
                PassportResponse::Profile(profile) => profile,
                // If the user canceled the authorization process, a redirect to i.e login page would be very convinient rather
                // than displaying some `Internal server error` just to say. It may not be exactly that kind of error, but can be inclusive of others.
                PassportResponse::FailureRedirect(failure) => {
                    return HttpResponse::SeeOther()
                        .append_header((http::header::LOCATION, failure.to_string()))
                        .finish()
                }
            };
            res
        }
        Err(error) => return HttpResponse::BadRequest().body(error.to_string()),
    };
    println!("out of the match expression");
    println!("Profile: {:#?}", profile);
    HttpResponse::Ok().json(profile)

    // if let Some(profile_array) = profile.as_array() {
    // 	println!("Profile: {:#?}", profile_array);
    // 	return HttpResponse::Ok().json(profile);

    // }

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
