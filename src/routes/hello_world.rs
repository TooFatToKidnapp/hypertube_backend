use actix_web::HttpResponse;

#[derive(serde::Serialize)]
struct ResponseMessage {
    message: String,
}

pub async fn handler() -> HttpResponse {
    HttpResponse::Ok().json(ResponseMessage {
        message: "Hello From Actix Server!!".to_string(),
    })
}
