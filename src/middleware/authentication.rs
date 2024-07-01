use std::{
    future::{ready, Ready},
    rc::Rc,
};
use uuid::Uuid;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Executor};
use tracing::Instrument;

pub struct Authentication {
    db_pool: PgPool
}

impl Authentication {
    pub fn new(db_pool: PgPool) -> Self {
        Authentication { db_pool }
    }
}

pub struct User {
    id: Uuid,
    username: String,
    email: String,
    created_at: String,
    updated_at: String
}


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

#[derive(Deserialize)]
pub struct Claims {
    pub id: String,
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
        let auth = req.headers().get(actix_web::http::header::AUTHORIZATION);
        if auth.is_none() {
            let http_res = HttpResponse::Unauthorized().json(json!({
                "Error" : "Missing access token"
            }));
            let (http_req, _) = req.into_parts();
            let res = ServiceResponse::new(http_req, http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }
        let authentication_token: String = auth.unwrap().to_str().unwrap_or("").to_string();

        if authentication_token.is_empty() {
            let http_res = HttpResponse::Unauthorized().json(json!({
                "Error" : "Invalid access token"
            }));
            let (http_req, _) = req.into_parts();
            let res = ServiceResponse::new(http_req, http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }

        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let token_result = decode::<Claims>(
            &authentication_token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        );

        let token = match token_result {
            Ok(token) => {
                token.claims
            },
            Err(_) => {
                let http_res = HttpResponse::Unauthorized().json(json!({
                    "Error" : "Invalid access token"
                }));
                let (http_req, _) = req.into_parts();
                let res = ServiceResponse::new(http_req, http_res);
                return (async move { Ok(res.map_into_right_body()) }).boxed_local();
            }
        };
        let db_connection = self.db_pool.clone();
        let fut = self.service.call(req);
        let uuid_result = Uuid::parse_str(token.id.as_str());
        let uuid = match uuid_result {
            Ok(uuid) => uuid,
            Err(err) => {
                let http_res = HttpResponse::Unauthorized().json(json!({
                    "Error" : "Invalid token content"
                }));
                let (http_req, _) = req.into_parts();
                let res = ServiceResponse::new(http_req, http_res);
                return (async move { Ok(res.map_into_right_body()) }).boxed_local();
            }
        };

        Box::pin(async move {

            let query_result = sqlx::query!(
                r#"
                    SELECT * FROM users WHERE id = $1
                "#,
                uuid,
            ).fetch_one(&db_connection)
            .instrument(query_span)
            .await;

            let user = match query_result {
                Ok(user) => {
                    tracing::info!("got user from token {:#?}", user);
                    User {
                        id: user.id,
                        username: user.username,
                        created_at: user.created_at.to_string(),
                        updated_at: user.updated_at.to_string(),
                        email: user.email
                    }
                }
                Err(err) => {
                    tracing::error!("Error getting user from database {}", err);
                    let http_res = HttpResponse::Unauthorized().json(json!({
                        "Error" : "Invalid access token"
                    }));
                    return Ok(http_res.map_into_left_body())
                }
            };



            let res: ServiceResponse<B> = fut.await?;
            Ok(res.map_into_left_body())
        })

    }
}
