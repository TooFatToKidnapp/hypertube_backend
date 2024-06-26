use std::net::TcpListener;
use hypertube_backend::configuration::{get_configuration, Settings};
use sqlx::{Connection, PgConnection, Executor ,PgPool};
use tokio;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn configure_database(config: &Settings) -> PgPool {
    println!("DB NAME : {}", config.database.connection_string_without_db().as_str());
    let mut connection =
        PgConnection::connect(config.database.connection_string_without_db().as_str())
            .await
            .expect("Failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, config.database.database_name).as_str())
        .await
        .expect("Failed to create database.");
		let connection_pool = PgPool::connect(config.database.connection_string().as_str()).await.expect("Failed to connect to postgres");
		sqlx::migrate!("./migrations")
			.run(&connection_pool)
			.await.expect("Failed to migrate the database");
		connection_pool
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("0.0.0.0:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    let mut configuration = get_configuration().expect("Failed ot read configuration file");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration).await;
		let server = hypertube_backend::startup::run_server(listener, connection_pool.clone()).expect("Failed to bind address");
		let _ = tokio::spawn(server);

		TestApp {
			address: format!("http://127.0.0.1:{}", port),
			db_pool: connection_pool
		}
}

