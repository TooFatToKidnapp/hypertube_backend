use crate::routes::{create_user, hello_world::handler};
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run_server(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(handler))
            .route("/user/create", web::post().to(create_user))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
