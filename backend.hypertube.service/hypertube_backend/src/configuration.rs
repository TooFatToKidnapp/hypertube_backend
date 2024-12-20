use config::{Config, File, FileFormat};
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings {
    pub user_name: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        env::var("DATABASE_URL").unwrap().to_string()
        // format!(
        //     "postgresql://{}:{}@{}:{}/{}",
        //     self.user_name, self.password, self.host, self.port, self.database_name
        // )
    }
}

pub fn get_configuration(filename: &str) -> Result<Settings, config::ConfigError> {
    let mut builder = Config::builder();
    builder = builder.add_source(File::new(filename, FileFormat::Json));
    let config = builder.build()?;
    config.try_deserialize()
}
