use hypertube_backend::startup;
use sqlx::PgPool;
use std::net::TcpListener;
use hypertube_backend::configuration::get_configuration;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8000").expect("Failed to bind");
    let configuration = get_configuration().expect("Failed to read `configuration.json`. Please make sure it exists and is valid JSON.");
    let connection_pool = PgPool::connect(configuration.database.connection_string().as_str())
        .await
        .expect("Failed to connect to database");
    startup::run_server(listener, connection_pool)?.await
}
