use std::rc::Rc;

use super::validate_uuid;
use crate::middleware::User;
use actix_web::{
    web::{Data, Path},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct RequestParam {
    #[validate(custom(function = "validate_uuid"))]
    pub id: String,
}

pub async fn get_user(
    req: HttpRequest,
    path: Path<RequestParam>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let query_span = tracing::info_span!("Get User info event");

    let is_valid = path.validate();
    if let Err(error) = is_valid {
        let source = error.field_errors();
        for i in source.iter() {
            for err in i.1.iter() {
                if let Some(message) = err.message.as_ref() {
                    tracing::error!("Error: {}", message.as_ref());
                    return HttpResponse::BadRequest().json(json!({
                        "Error" : message.as_ref()
                    }));
                }
            }
        }
        return HttpResponse::BadRequest().finish();
    }

    let visitor_id = {
        let extension = req.extensions();
        let user_option = extension.get::<Rc<User>>();
        match user_option {
            Some(user) => user.id,
            None => {
                tracing::info!("User field not found in req object");
                return HttpResponse::NotFound().json(json!({
                    "error": "user not found"
                }));
            }
        }
    };

    let parsed_user_id = match path.id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(err) => {
            tracing::error!("Error parsing param id {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "Error parsing param id"
            }));
        }
    };

    let query_res = sqlx::query!(
        r#"
            SELECT * FROM users WHERE id = $1
        "#,
        parsed_user_id.clone()
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span)
    .await;

    let user_info = match query_res {
        Ok(user) => User {
            session_id: None,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            image_url: user.profile_picture_url,
            id: user.id,
            created_at: user.created_at.to_string(),
            updated_at: user.updated_at.to_string(),
        },
        Err(sqlx::Error::RowNotFound) => {
            tracing::error!("User not found");
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            tracing::error!("Database Error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };

    let mut response_body = json!({
        "data" : {
            "id": user_info.id.to_string(),
            "first_name": user_info.first_name,
            "last_name": user_info.last_name,
            "created_at": user_info.created_at.to_string(),
            "updated_at": user_info.updated_at.to_string(),
            "username" : user_info.username,
            "image_url": user_info.image_url
        }
    });

    if parsed_user_id == visitor_id {
        response_body["data"]["email"] = user_info.email.into();
    }

    HttpResponse::Ok().json(response_body)
}
