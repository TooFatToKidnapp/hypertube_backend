use actix_web::{HttpResponse, HttpRequest};
use serde_json::json;

pub async fn check_session(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "authenticated": true
    }))
}