use actix_web::HttpResponse;
use serde_json::json;

pub async fn handler() -> HttpResponse {
    tracing::info!("Hello World Handler");
    HttpResponse::Ok().json(json!({
        "message": "Hello From Actix Server!!"
    }))
}
