use std::rc::Rc;

use sqlx::PgPool;
use actix_web::{http, web::{Data, Path}, HttpRequest, HttpResponse};
use uuid::Uuid;


#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub image_url: Option<String>,
    pub username: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
    pub session_id: Option<Uuid>,
}


use super::Source;

pub async fn get_favorite_movies(
    path: Path<(String, Source)>,
    connection: Data<PgPool>,
    req:HttpRequest,
    // query_span: Span,
) -> HttpResponse {
    if let Some(user) = req.extensions().get::<Rc<User>>() {
        // Use the user as needed
        println!("User ID: {}", user.id);
        println!("User Name: {}", user.name);
        
        // Example response using user information
        HttpResponse::Ok().body(format!("Hello, {}!", user.name))
    } else {
        HttpResponse::Unauthorized().body("User not found")
    }
    // let query_res = sqlx.
}