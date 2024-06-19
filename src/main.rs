use actix_web::{web, App, HttpResponse, HttpServer};
use std::net::TcpListener;

#[derive(serde::Serialize)]
struct ResponseMessage {
    message: String
}


async fn handler() -> HttpResponse {
    HttpResponse::Ok().json(ResponseMessage {
        message: "Hello From Actix Server!!".to_string()
    })
}

#[actix_web::main]
async fn main()-> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8000").expect(
        "Failed to bind"
    );
    let _server = HttpServer::new(|| {
        App::new()
        .route("/", web::get().to(handler))
    })
    .listen(listener)?
    .run()
    .await;

    Ok(())
}
