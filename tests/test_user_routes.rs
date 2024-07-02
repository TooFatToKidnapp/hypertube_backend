mod test_startup;
use std::borrow::Borrow;

use serde::{Deserialize, Serialize};
use test_startup::*;

#[derive(Clone, Serialize, Debug)]
struct SignUpBody<'a> {
    email: &'a str,
    username: &'a str,
    password: &'a str,
}

#[derive(Serialize, Deserialize)]
struct SignUpResponse {
    token: String,
    email: String,
    created_at: String,
    updated_at: String,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct UserInfoResponse {
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
    let response = res
        .json::<Data<SignUpResponse>>()
        .await
        .expect("Failed to parse response body");

    response.data.token
}

#[actix_rt::test]
async fn get_user_test() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let server_address = format!("{}/user", app.address.as_str());
    let body = SignUpBody {
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/user/sign-up", app.address.as_str());
    let token = create_temporary_user(adder, body.clone()).await;

    let response = client
        .get(server_address)
        .header("Authorization", format!("Bearer {}", token))
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
    let response_body = response
        .json::<Data<SignUpResponse>>()
        .await
        .expect("Failed to parse response body");
    assert_eq!(response_body.data.email, login_body.email);
    assert!(!response_body.data.token.is_empty());
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
