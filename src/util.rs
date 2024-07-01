use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize)]
pub struct ResponseMessage {
    pub message: String,
}

impl ResponseMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub fn check_for_necessary_env() {
    env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");
    env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::var("JWT_SECRET").expect("JWT_SECRET must be set");
}
