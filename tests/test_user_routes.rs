mod test_startup;

use actix_web::http;
use reqwest;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use test_startup::*;

#[derive(Clone, Serialize, Debug)]
struct SignUpBody<'a> {
    email: &'a str,
    username: &'a str,
    password: &'a str,
}

#[derive(Serialize, Deserialize)]
struct SignUpResponse {
    email: String,
    created_at: String,
    updated_at: String,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct UserInfoResponse {
    id: String,
    email: String,
    username: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct LoginBody<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize, Deserialize)]
struct Data<T> {
    data: T,
}

async fn create_temporary_user<'a>(address: String, body: SignUpBody<'a>) -> String {
    let client = reqwest::Client::new();
    let res = client
        .post(address.as_str())
        .json(body.borrow())
        .send()
        .await
        .expect("Failed to execute request");

    assert!(res.status().is_success());
    let mut session_id = String::new();
    for cookie in res.cookies().into_iter() {
        if cookie.name() == "session" {
            cookie.value().clone_into(&mut session_id);
        }
    }
    assert!(!session_id.is_empty());
    session_id
}

#[actix_rt::test]
async fn get_user_with_invalid_cookie() {
    let app = spawn_app().await;
    let server_address = format!("{}/user", app.address.as_str());
    let body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder, body.clone()).await;
    let client = reqwest::Client::new();

    let response = client
        .get(server_address)
        .header(http::header::COOKIE, format!("session={}", "XXXXXXXXX"))
        .send()
        .await
        .expect("Failed to send request");
    assert!(response.status().is_client_error());
}

#[actix_rt::test]
async fn get_user_test() {
    let app = spawn_app().await;
    let server_address = format!("{}/user", app.address.as_str());
    let body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let session_id = create_temporary_user(adder, body.clone()).await;
    let client = reqwest::Client::new();

    let response = client
        .get(server_address)
        .header(http::header::COOKIE, format!("session={}", session_id))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());

    let response = response
        .json::<Data<UserInfoResponse>>()
        .await
        .expect("Failed to parse response body");
    assert_eq!(response.data.email, body.email);
    assert_eq!(response.data.username, body.username);
}

#[actix_rt::test]
async fn test_user_already_exists() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder.clone(), body.clone()).await;

    let response = client
        .post(adder)
        .json(&body)
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_client_error());
}

#[actix_rt::test]
async fn user_login_test() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let signup_body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let login_body = LoginBody {
        email: "test@gmail.com",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder.clone(), signup_body).await;

    let login_addr = format!("{}/user/login", app.address.as_str());
    let response = client
        .post(login_addr)
        .json(&login_body)
        .send()
        .await
        .expect("Failed to send request");
    assert!(response.status().is_success());
    let headers = response.headers();
    let set_cookies = headers.get("Set-Cookie").unwrap();
    let cookie_jar = set_cookies
        .to_str()
        .unwrap()
        .split(';')
        .collect::<Vec<&str>>();
    let mut session_value = String::new();
    for cookie in cookie_jar {
        if cookie.trim().starts_with("session=") {
            cookie
                .trim()
                .split('=')
                .nth(1)
                .unwrap_or_default()
                .clone_into(&mut session_value);
        }
    }
    assert!(!session_value.is_empty());
    let response_body = response
        .json::<Data<SignUpResponse>>()
        .await
        .expect("Failed to parse response body");
    assert_eq!(response_body.data.email, login_body.email);
}

#[actix_rt::test]
async fn user_sign_up_test() {
    let app = spawn_app().await;
    let signup_body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder.clone(), signup_body).await;
}
