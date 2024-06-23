use crate::util::ResponseMessage;
use actix_web::HttpResponse;

pub async fn handler() -> HttpResponse {
    tracing::info!("Hello World Handler");
    HttpResponse::Ok().json(ResponseMessage {
        message: "Hello From Actix Server!!".to_string(),
    })
}
