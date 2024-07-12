mod test_startup;

use actix_web::http;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Borrow;
use test_startup::*;

#[derive(Clone, Serialize, Debug)]
struct SignUpBody<'a> {
    email: &'a str,
    first_name: &'a str,
    last_name: &'a str,
    username: &'a str,
    password: &'a str,
}

#[derive(Serialize, Deserialize)]
struct SignUpResponse {
    email: String,
    created_at: String,
    first_name: Option<String>,
    last_name: Option<String>,
    updated_at: String,
    username: String,
    id: String,
}

#[derive(Serialize, Deserialize)]
struct UserInfoResponse {
    id: String,
    email: String,
    image_url: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
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

async fn create_temporary_user<'a>(address: String, body: SignUpBody<'a>) -> (String, String) {
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
    let id = res
        .json::<Data<SignUpResponse>>()
        .await
        .expect("Failed to parse response")
        .data
        .id;
    (session_id, id)
}

#[actix_rt::test]
async fn get_user_with_invalid_cookie() {
    let app = spawn_app().await;
    let server_address = format!("{}/users", app.address.as_str());
    let body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
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
    let body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let (session_id, user_id) = create_temporary_user(adder, body.clone()).await;
    let server_address = format!("{}/users/{}", app.address.as_str(), user_id);
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
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
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
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let login_body = LoginBody {
        email: "test@gmail.com",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder.clone(), signup_body).await;

    let login_addr = format!("{}/users/login", app.address.as_str());
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
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder.clone(), signup_body).await;
}

#[derive(Serialize)]
struct ResetPassword<'a> {
    old_password: &'a str,
    new_password: &'a str,
}

#[actix_rt::test]
async fn reset_user_password_reset() {
    let app = spawn_app().await;
    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let (session_id, _) = create_temporary_user(
        format!("{}/users/sign-up", app.address.as_str()),
        signup_body,
    )
    .await;

    let client = reqwest::Client::new();

    let request_body = ResetPassword {
        old_password: "Password@123",
        new_password: "Test@123456",
    };

    let res = client
        .post(format!("{}/users/password/update", app.address.as_str()))
        .header("Cookie", format!("session={}", session_id))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_success());
}

#[actix_rt::test]
async fn reset_user_password_reset_invalid() {
    let app = spawn_app().await;
    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let (session_id, _) = create_temporary_user(
        format!("{}/users/sign-up", app.address.as_str()),
        signup_body,
    )
    .await;

    let client = reqwest::Client::new();

    let request_body = ResetPassword {
        old_password: "Password@123",
        new_password: "---",
    };

    let res = client
        .post(format!("{}/users/password/update", app.address.as_str()))
        .header("Cookie", format!("session={}", session_id))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_client_error());
}

#[actix_rt::test]
async fn test_log_out_user() {
    let app = spawn_app().await;

    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let (session_id, _) = create_temporary_user(adder, signup_body).await;
    let sign_out_adder = format!("{}/users/sign-out", app.address.as_str());

    let client = reqwest::Client::new();

    let res = client
        .get(sign_out_adder.clone())
        .header(http::header::COOKIE, format!("session={}", session_id))
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_success());
    let sign_out_session_val = res.headers().get("Set-Cookie").unwrap().to_str().unwrap();
    assert_eq!(
        sign_out_session_val,
        "session=; HttpOnly; SameSite=Strict; Secure; Path=/; Max-Age=0"
    );
}

#[actix_rt::test]
async fn test_user_logout_with_invalid_session() {
    let app = spawn_app().await;

    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let _ = create_temporary_user(adder, signup_body).await;

    let sign_out_adder = format!("{}/users/sign-out", app.address.as_str());

    let client = reqwest::Client::new();

    let res = client
        .get(sign_out_adder)
        .header(
            http::header::COOKIE,
            format!("session={}", uuid::Uuid::new_v4().to_string()),
        )
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_client_error());
}

#[actix_rt::test]
async fn test_user_profile_edit_route() {
    let app = spawn_app().await;

    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let (session_id, _) = create_temporary_user(adder, signup_body).await;

    let edit_profile_adder = format!("{}/users/update", app.address.as_str());

    let client = reqwest::Client::new();

    let res = client
        .patch(edit_profile_adder)
        .header(http::header::COOKIE, format!("session={}", session_id))
        .json(&json!({
            "first_name" : "example1",
            "last_name": "example2",
            "email": None::<String>,
            "username": None::<String>
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_success());
    let response = res
        .json::<serde_json::Value>()
        .await
        .expect("Failed to parse response body");

    assert_eq!(response["data"]["first_name"].as_str(), Some("example1"));
    assert_eq!(response["data"]["last_name"].as_str(), Some("example2"));
    assert_eq!(response["data"]["email"].as_str(), Some("test@gmail.com"));
    assert_eq!(response["data"]["username"].as_str(), Some("username123"));
}

#[actix_rt::test]
async fn test_user_profile_edit_route_with_used_email() {
    let app = spawn_app().await;

    let signup_body = SignUpBody {
        first_name: "test first name",
        last_name: "test last name",
        email: "test@gmail.com",
        username: "username123",
        password: "Password@123",
    };
    let adder = format!("{}/users/sign-up", app.address.as_str());
    let (session_id, _) = create_temporary_user(adder, signup_body).await;

    let edit_profile_adder = format!("{}/users/update", app.address.as_str());

    let client = reqwest::Client::new();

    let res = client
        .patch(edit_profile_adder)
        .header(http::header::COOKIE, format!("session={}", session_id))
        .json(&json!({
            "first_name" : "example1",
            "last_name": "example2",
            "email": "test@gmail.com",
            "username": None::<String>
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(res.status().is_client_error());
}
