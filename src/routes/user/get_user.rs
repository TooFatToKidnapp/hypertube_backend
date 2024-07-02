use std::rc::Rc;

use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use serde_json::json;

use crate::middleware::User;

pub async fn get_user(req: HttpRequest) -> HttpResponse {
    let extension = req.extensions();
    let user_option = extension.get::<Rc<User>>();
    match user_option {
        Some(user) => {
            tracing::info!("sending user info");
            HttpResponse::Ok().json(json!({
                "data" : {
                    "email": user.email,
                    "username": user.username,
                    "created_at": user.created_at,
                    "updated_at": user.updated_at
                }
            }))
        }
        None => {
            tracing::info!("User field not found in req object");
            HttpResponse::NotFound().json(json!({
                "error": "user not found"
            }))
        }
    }
}
