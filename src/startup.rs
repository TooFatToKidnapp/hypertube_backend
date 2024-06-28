use crate::routes::{create_user, hello_world::handler};
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use actix_cors::Cors;
use actix_web::http::header;

use std::env;

fn configure_cors(frontend_url: &str) -> Cors {
    if frontend_url == "*" {
        Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600)
    } else {
        Cors::default()
            .allowed_origin(frontend_url)
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600)
    }
}

pub fn run_server(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let frontend_url = env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");

    let server = HttpServer::new(move || {
        let cors = configure_cors(&frontend_url);

        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .route("/", web::get().to(handler))
            .route("/user/create", web::post().to(create_user))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
