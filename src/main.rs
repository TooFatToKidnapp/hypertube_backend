use std::net::TcpListener;
use hypertube_backend::startup;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8000").expect("Failed to bind");
    startup::run_server(listener)?.await
}
