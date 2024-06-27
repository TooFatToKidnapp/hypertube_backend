use hypertube_backend::configuration::{get_configuration, Settings};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use tokio::task;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
    pub user_name: String,
    pub password: String,
    pub host: String,
    pub parent_db_name: String,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let connection_url = format!(
            "postgresql://{}:{}@{}/{}",
            self.user_name, self.password, self.host, self.parent_db_name
        );
        task::spawn_blocking(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut connection = PgConnection::connect(connection_url.as_str())
                    .await
                    .expect("Failed to connect to Postgres for cleanup");
                connection
                    .execute(format!(r#"DROP DATABASE "{}""#, db_name).as_str())
                    .await
                    .expect("Failed to drop database.");
            });
        });
    }
}

pub async fn configure_database(config: &Settings, parent_db_name: &str) -> PgPool {
    let url = format!(
        "postgresql://{}:{}@{}/{}",
        config.database.user_name, config.database.password, config.database.host, parent_db_name
    );
    let mut connection = PgConnection::connect(url.as_str())
        .await
        .expect("Failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, config.database.database_name).as_str())
        .await
        .expect("Failed to create database.");
    let connection_pool = PgPool::connect(url.as_str())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("0.0.0.0:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    let mut configuration =
        get_configuration("test_configuration").expect("Failed to read configuration file");
    let parent_db_name = configuration.database.database_name.clone();
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration, &parent_db_name.to_string()).await;
    let server = hypertube_backend::startup::run_server(listener, connection_pool.clone())
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: connection_pool,
        db_name: configuration.database.database_name,
        user_name: configuration.database.user_name,
        password: configuration.database.password,
        host: configuration.database.host,
        parent_db_name: parent_db_name.to_string(),
    }
}
