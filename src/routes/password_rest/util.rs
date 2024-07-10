use super::{reset_password, send_password_reset_email, validate_password_reset_code};
use actix_web::{web, Scope};

pub fn password_source() -> Scope {
    web::scope("/password")
        .route("/email", web::post().to(send_password_reset_email))
        .route(
            "/validate/code",
            web::post().to(validate_password_reset_code),
        )
        .route("/update", web::patch().to(reset_password))
}
