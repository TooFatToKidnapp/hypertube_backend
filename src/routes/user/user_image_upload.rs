use actix_multipart::Multipart;
use actix_web::{
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};
use chrono::Utc;
use futures_util::TryStreamExt;
use mime::{IMAGE_JPEG, IMAGE_PNG};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::User;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    config::{self, Credentials},
    primitives::ByteStream,
    Client,
};
use std::{env, rc::Rc};
use tracing::Instrument;

const MAX_FILE_SIZE: usize = 5_000_600; // 5 mb

fn get_aws_client() -> Result<Client, ()> {
    let id = env::var("S3_BUCKET_ACCESS_KEY").unwrap();
    let secret = env::var("S3_BUCKET_SECRET_KEY").unwrap();
    let provider = env::var("S3_PROVIDER_URL").unwrap();
    let region = env::var("S3_REGION").unwrap();

    let cred = Credentials::new(id, secret, None, None, "loaded up from env");

    let region = Region::new(region.to_string());
    let conf_builder = config::Builder::new()
        .region(region)
        .credentials_provider(cred)
        .endpoint_url(provider)
        .behavior_version(BehaviorVersion::latest());
    let conf = conf_builder.build();
    let client = Client::from_conf(conf);
    Ok(client)
}

pub async fn upload_user_profile_image(
    mut payload: Multipart,
    connection: Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let query_span = tracing::info_span!("Saving user profile image in the database");
    let (user_email, user_image_url) = {
        let extension = req.extensions();
        match extension.get::<Rc<User>>() {
            Some(user) => (user.email.clone(), user.image_url.clone()),
            None => {
                tracing::info!("User field not found in req object");
                return HttpResponse::NotFound().json(json!({
                        "error": "user not found"
                }));
            }
        }
    };

    let content_len = match req.headers().get("Content-Length") {
        Some(value) => {
            let len_res = value.to_str().unwrap_or("0").parse::<usize>();
            if len_res.is_err() {
                tracing::error!("Invalid Content-Length value");
                return HttpResponse::BadRequest().finish();
            }
            let len = len_res.unwrap();
            if len == 0 || len > MAX_FILE_SIZE {
                tracing::error!("Invalid Content-Length value [{}]", len);
                return HttpResponse::BadRequest().finish();
            }
            len
        }
        None => {
            tracing::error!("no Content-Length header found");
            return HttpResponse::BadRequest().finish();
        }
    };

    tracing::info!("Got Content-Length {}", content_len);

    let image_url = if let Ok(Some(mut field)) = payload.try_next().await {
        if field.name().is_none() || field.name().unwrap() != "image" {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid field name"
            }));
        }

        let file_type_res = field.content_type();

        let file_type = match file_type_res {
            Some(extension) => {
                if *extension == IMAGE_JPEG {
                    "image/jpeg"
                } else if *extension == IMAGE_PNG {
                    "image/png"
                } else {
                    tracing::error!("Wrong file type");
                    return HttpResponse::BadRequest().json(json!({
                        "error": "Invalid file type"
                    }));
                }
            }
            None => {
                tracing::error!("Didn't get file type");
                return HttpResponse::BadRequest().json(json!({
                    "error": "Invalid file type"
                }));
            }
        };

        let aws_client = match get_aws_client() {
            Ok(client) => {
                tracing::info!("Got Client");
                client
            }
            Err(_) => {
                tracing::error!("Failed to create aws client");
                return HttpResponse::InternalServerError().json(json!({
                    "error": "Something went wrong"
                }));
            }
        };

        let file_name = format!(
            "{}-{}.{}",
            chrono::Utc::now(),
            Uuid::new_v4(),
            file_type.split('/').last().unwrap()
        )
        .replace(' ', "-");

        let bucket = env::var("S3_BUCKET_NAME").unwrap();
        let key: String = format!("profile/{}", file_name);
        let mut file_bytes = web::BytesMut::new();

        while let Ok(Some(chunk)) = field.try_next().await {
            if file_bytes.len() + chunk.len() > MAX_FILE_SIZE {
                tracing::error!("File is too large");
                return HttpResponse::BadRequest().json(json!({
                        "error": "File is too large"
                }));
            }
            file_bytes.extend_from_slice(&chunk);
        }

        let body = ByteStream::from(file_bytes.freeze());

        if user_image_url.is_some() {
            tracing::info!("User already has a profile image. Delaying...");
            let url = user_image_url.unwrap();
            let path = url.split("/profile/").collect::<Vec<&str>>()[1];
            match aws_client
                .delete_object()
                .bucket(bucket.clone())
                .key(format!("profile/{}", path))
                .send()
                .await
            {
                Ok(_) => tracing::info!("File Deleted successfully"),
                Err(err) => {
                    tracing::error!("Failed to delete file: {:?}", err);
                    return HttpResponse::InternalServerError().json(json!({
                        "error": "Something went wrong"
                    }));
                }
            }
        }

        match aws_client
            .put_object()
            .bucket(bucket)
            .key(&key)
            .body(body)
            .content_type(file_type)
            .acl("public-read".into())
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!("File saved successfully");
                format!(
                    "{}/profile/{}",
                    env::var("S3_BUCKET_URL").unwrap(),
                    file_name
                )
            }
            Err(err) => {
                tracing::error!("Failed to save file: {:?}", err);
                return HttpResponse::InternalServerError().json(json!({
                    "error": "Something went wrong"
                }));
            }
        }
    } else {
        tracing::error!("Not file in request");
        return HttpResponse::BadRequest().json(json!({
            "error": "No file in request"
        }));
    };

    tracing::info!("File uploaded Successfully");

    let query_res = sqlx::query(
        r#"
				Update users SET profile_picture_url = $1, updated_at = $2 WHERE email = $3
			"#,
    )
    .bind(image_url)
    .bind(Utc::now())
    .bind(user_email)
    .execute(connection.as_ref())
    .instrument(query_span)
    .await;

    match query_res {
        Ok(_) => {
            tracing::info!("image saved successfully in the database");
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!("Database error {:#?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "something went wrong"
            }))
        }
    }
}
