#[derive(serde::Serialize)]
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
