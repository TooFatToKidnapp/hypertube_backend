use crate::passport::{generate_passports, passport_route_redirect, passport_oauth};
use crate::routes::hello_world::handler;
use crate::routes::movies::movie_source;
use crate::routes::password_rest::password_source;
use crate::routes::user::user_source;
use crate::routes::{comment_source, CronJobScheduler};

use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use sqlx::PgPool;
use std::net::TcpListener;
use tokio::sync::RwLock;
use tracing_actix_web::TracingLogger;

use actix_cors::Cors;
use actix_web::http::header;
use dotenv::dotenv;
use std::env;

fn configure_cors(frontend_url: &str) -> Cors {
    let mut cors = Cors::default();
    cors = if frontend_url == "*" {
        cors.allow_any_origin()
    } else {
        cors.allowed_origin(frontend_url)
    };
    cors.allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
        .supports_credentials()
}

pub fn run_server(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    dotenv().ok();
    let db_pool = Data::new(db_pool);
    let passport_state =
        Data::new(RwLock::new(generate_passports()?));
    let cron_task_handler = Data::new(CronJobScheduler::new());
    let frontend_url = env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");

    let server: Server = HttpServer::new(move || {
        let cors = configure_cors(frontend_url.as_str());
        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .app_data(cron_task_handler.clone())
            .app_data(passport_state.clone())
            .service(passport_oauth())
            .service(passport_route_redirect())
            .service(comment_source(&db_pool))
            .service(user_source(&db_pool))
            .service(password_source())
            .service(movie_source(&db_pool))
            .route("/", web::get().to(handler))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
