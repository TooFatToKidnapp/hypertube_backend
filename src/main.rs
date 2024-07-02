use dotenv::dotenv;
use hypertube_backend::configuration::get_configuration;
use hypertube_backend::startup;
use hypertube_backend::telemetry::{get_subscriber, init_subscriber};
use hypertube_backend::util::check_for_necessary_env;
use sqlx::PgPool;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    check_for_necessary_env();
    let subscriber = get_subscriber("hyper_tube", "info", std::io::stdout);
    init_subscriber(subscriber);
    let listener = TcpListener::bind("0.0.0.0:8000").expect("Failed to bind");
    let configuration = get_configuration("configuration").expect(
        "Failed to read `configuration.json`. Please make sure it exists and is valid JSON.",
    );
    let connection_pool = PgPool::connect(configuration.database.connection_string().as_str())
        .await
        .expect("Failed to connect to database");
    startup::run_server(listener, connection_pool)?.await
}
