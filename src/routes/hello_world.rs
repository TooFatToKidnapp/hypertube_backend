use actix_web::HttpResponse;
use crate::util::ResponseMessage;

pub async fn handler() -> HttpResponse {
    HttpResponse::Ok().json(ResponseMessage {
        message: "Hello From Actix Server!!".to_string(),
    })
}
