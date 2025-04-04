use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};

use futures_util::{future::LocalBoxFuture, FutureExt};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::Instrument;
use uuid::Uuid;

pub struct Authentication {
    db_pool: PgPool,
}

impl Authentication {
    pub fn new(db_pool: PgPool) -> Self {
        Authentication { db_pool }
    }
}

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
    pub profile_is_finished: bool,
    pub password_is_set: bool,
}

// https://imfeld.dev/writing/actix-web-middleware

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Rc::new(service),
            db_pool: self.db_pool.clone(),
        }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: Rc<S>,
    db_pool: PgPool,
}

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub id: String,
    pub exp: usize,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Error = Error;
    type Response = ServiceResponse<EitherBody<B>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let query_span = tracing::info_span!("Authentication middleware");
        let session_value: String = {
            let cookies_res = req.cookies();
            let cookie_jar = match cookies_res {
                Ok(cookies) => {
                    tracing::info!("Got cookies");
                    cookies
                }
                Err(_) => {
                    drop(cookies_res);
                    tracing::error!("No Cookies found in request");
                    let http_res = HttpResponse::Unauthorized().json(json!({
                        "Error" : "No Cookies found in request"
                    }));
                    let (http_req, _) = req.into_parts();
                    let res = ServiceResponse::new(http_req, http_res);
                    return (async move { Ok(res.map_into_right_body()) }).boxed_local();
                }
            };
            let mut value = String::new();
            for cookie in cookie_jar.iter() {
                if cookie.name() == "session" {
                    cookie.value().clone_into(&mut value);
                    break;
                }
            }
            value
        };

        if session_value.is_empty() {
            tracing::error!("No Session Cookie found in request");
            let http_res = HttpResponse::Unauthorized().json(json!({
                "Error" : "No Session Cookie found in request"
            }));
            let (http_req, _) = req.into_parts();
            let res = ServiceResponse::new(http_req, http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }
        let uuid_result = Uuid::parse_str(session_value.as_str());
        if uuid_result.is_err() {
            tracing::error!("Invalid Cookie value");
            let http_res = HttpResponse::Unauthorized().json(json!({
                "Error" : "Invalid Cookie value"
            }));
            let (http_req, _) = req.into_parts();
            let res = ServiceResponse::new(http_req, http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }
        let session = uuid_result.unwrap();
        let db_connection = self.db_pool.clone();
        let service = self.service.clone();
        async move {
            let session_query_res = sqlx::query!(
                r#"
                    SELECT * FROM sessions WHERE id = $1
                "#,
                session
            )
            .fetch_one(&db_connection)
            .instrument(query_span.clone())
            .await;

            let session = match session_query_res {
                Ok(session) => {
                    tracing::info!("USER ID: {}", session.user_id);
                    tracing::info!("Found session");
                    session
                }
                Err(sqlx::Error::RowNotFound) => {
                    tracing::error!("Session not found");
                    let http_res = HttpResponse::BadRequest().json(json!({
                        "Error" : "invalid Session"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
                Err(err) => {
                    tracing::error!("Database error {}", err);
                    let http_res = HttpResponse::BadRequest().json(json!({
                        "Error" : "Database Error"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
            };

            if session.expires_at < chrono::Utc::now() {
                tracing::info!("Session EXPIRED!");
                let delete_res = sqlx::query("DELETE FROM sessions WHERE id = $1")
                    .bind(session.id)
                    .execute(&db_connection)
                    .instrument(query_span.clone())
                    .await;
                if delete_res.is_err() {
                    tracing::error!("Database error {}", delete_res.unwrap_err());
                    let http_res = HttpResponse::BadRequest().json(json!({
                        "Error" : "Database Error"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
                tracing::error!("Session Expired");
                let http_res = HttpResponse::BadRequest().json(json!({
                    "Error" : "Session Expired"
                }));
                let (http_req, _) = req.into_parts();
                let response = ServiceResponse::new(http_req, http_res);
                return Ok(response.map_into_right_body());
            }

            let query_result = sqlx::query!(
                r#"
                    SELECT * FROM users WHERE id = $1
                "#,
                session.user_id,
            )
            .fetch_one(&db_connection)
            .instrument(query_span)
            .await;

            let user = match query_result {
                Ok(user) => {
                    tracing::info!("got user from session {:#?}", user);
                    User {
                        id: user.id,
                        first_name: user.first_name,
                        last_name: user.last_name,
                        image_url: user.profile_picture_url,
                        username: user.username,
                        created_at: user.created_at.to_string(),
                        updated_at: user.updated_at.to_string(),
                        email: user.email,
                        session_id: Some(session.id),
                        profile_is_finished: user.profile_is_finished,
                        password_is_set: user.password_is_set,
                    }
                }
                Err(sqlx::Error::RowNotFound) => {
                    tracing::error!("USER NOT FOUND IN DATABASE");
                    let http_res = HttpResponse::NotFound().json(json!({
                        "Error" : "user not found"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
                Err(err) => {
                    tracing::error!("Error getting user from database {}", err);
                    let http_res = HttpResponse::Unauthorized().json(json!({
                        "Error" : "database error"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
            };

            tracing::info!("INSERTING USER: {:#?}", user);
            req.extensions_mut().insert::<Rc<User>>(Rc::new(user));

            if let Some(user) = req.extensions().get::<Rc<User>>() {
                tracing::debug!("User found M: {:?}", user);
                // HttpResponse::Ok().body(format!("Hello, {}!", user.username))
            } else {
                tracing::debug!("User not found in request extensions M");
                // HttpResponse::Unauthorized().body("User not found")
            }

            let fut = service.call(req);
            let res: ServiceResponse<B> = fut.await?;
            Ok(res.map_into_left_body())
        }
        .boxed_local()
    }
}
