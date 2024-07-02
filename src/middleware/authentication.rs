use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Executor, PgPool};
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

pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
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
        let parts: Vec<&str> = authentication_token.split_ascii_whitespace().collect();

        if parts.len() != 2 || (parts[0] != "Bearer" && parts[0] != "bearer") || parts[1].is_empty()
        {
            tracing::error!(
                "Invalid AUTHORIZATION header len = {}, [0] = [{}], [1] = [{}]",
                parts.len(),
                parts[0],
                parts[1]
            );
            let http_res = HttpResponse::Unauthorized().json(json!({
                "Error" : "Invalid access token"
            }));
            let (http_req, _) = req.into_parts();
            let res = ServiceResponse::new(http_req, http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }
        let authentication_token = parts[1];
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let token_result = decode::<Claims>(
            authentication_token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        );

        let token = match token_result {
            Ok(token) => {
                tracing::info!("token decoded sccussefuly");
                token.claims
            }
            Err(_) => {
                tracing::error!("Error decoding token");
                let http_res = HttpResponse::Unauthorized().json(json!({
                    "Error" : "Invalid access token"
                }));
                let (http_req, _) = req.into_parts();
                let res = ServiceResponse::new(http_req, http_res);
                return (async move { Ok(res.map_into_right_body()) }).boxed_local();
            }
        };
        let db_connection = self.db_pool.clone();
        let uuid_result = Uuid::parse_str(token.id.as_str());
        let uuid = match uuid_result {
            Ok(uuid) => uuid,
            Err(_) => {
                tracing::error!("Invalid uuid");
                let http_res = HttpResponse::Unauthorized().json(json!({
                    "Error" : "Invalid token content"
                }));
                let (http_req, _) = req.into_parts();
                let res = ServiceResponse::new(http_req, http_res);
                return (async move { Ok(res.map_into_right_body()) }).boxed_local();
            }
        };
        let service = self.service.clone();
        // let fut = self.service.call(req);
        async move {
            let query_result = sqlx::query!(
                r#"
                    SELECT * FROM users WHERE id = $1
                "#,
                uuid,
            )
            .fetch_one(&db_connection)
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
                        email: user.email,
                    }
                }
                Err(err) => {
                    tracing::error!("Error getting user from database {}", err);
                    let http_res = HttpResponse::Unauthorized().json(json!({
                        "Error" : "Invalid access token"
                    }));
                    let (http_req, _) = req.into_parts();
                    let response = ServiceResponse::new(http_req, http_res);
                    return Ok(response.map_into_right_body());
                }
            };

            req.extensions_mut().insert::<Rc<User>>(Rc::new(user));
            let fut = service.call(req);
            let res: ServiceResponse<B> = fut.await?;
            Ok(res.map_into_left_body())
        }
        .boxed_local()
    }
}
