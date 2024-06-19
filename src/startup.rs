use actix_web::{dev::Server, web, App, HttpServer};
use std::net::TcpListener;

use crate::routes::hello_world::*;

pub fn run_server(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/", web::get().to(handler)))
        .listen(listener)?
        .run();

    Ok(server)
}
