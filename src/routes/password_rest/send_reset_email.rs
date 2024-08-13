use crate::middleware::User;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use chrono::Utc;
use lettre::{
    message::{Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};

use rand::{self, Rng};
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tracing::Instrument;
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct UserEmail {
    #[validate(email(message = "Not a valid email"))]
    pub email: String,
}

fn generate_random_ten_character_code() -> String {
    rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(10)
        .map(char::from)
        .collect()
}

fn build_email(verification_code: &str, username: &str) -> String {
    let style = "
        <style>
            body {
                font-family: Arial, sans-serif;
                background-color: #f4f4f4;
                margin: 0;
                padding: 0;
            }
            .container {
                width: 100%;
                max-width: 600px;
                margin: 0 auto;
                background-color: #ffffff;
                padding: 20px;
                border-radius: 8px;
                box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
            }
            .header {
                text-align: center;
                padding: 10px 0;
            }
            .header h1 {
                margin: 0;
                font-size: 24px;
                color: #333333;
            }
            .content {
                margin: 20px 0;
                font-size: 16px;
                line-height: 1.6;
                color: #333333;
            }
            .content p {
                margin: 0 0 10px;
            }
            .verification-code {
                display: block;
                width: fit-content;
                margin: 20px auto;
                padding: 10px 20px;
                font-size: 24px;
                color: #ffffff;
                background-color: #4CAF50;
                border-radius: 4px;
                text-align: center;
            }
            .footer {
                text-align: center;
                margin-top: 20px;
                font-size: 14px;
                color: #777777;
            }
        </style>
    ";

    format!(
        r##"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                {}
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>Reset your password?</h1>
                    </div>
                    <div class="content">
                        <p>Hello,</p>
                        <p>We received a request to reset your password. Use the verification code below to reset your password. If you did not request a password reset, please ignore this email.</p>
                        <p>the verification code is only valid for 10 minutes</p>
                        <div class="verification-code">{}</div>
                    </div>
                    <div class="footer">
                        <p>This email was meant for {}</p>
                    </div>
                </div>
            </body>
            </html>
        "##,
        style, verification_code, username
    )
}

fn send_email(email_content: String, email: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sender = std::env::var("EMAIL_SENDER_USERNAME").expect("EMAIL_SENDER_USERNAME not set");
    let sender_password =
        std::env::var("EMAIL_SENDER_PASSWORD").expect("EMAIL_SENDER_PASSWORD not set");

    let email_body = Message::builder()
        .from(sender.parse::<Mailbox>()?)
        .to(email.parse::<Mailbox>()?)
        .subject("Password reset request")
        .multipart(MultiPart::alternative().singlepart(SinglePart::html(email_content)))?;

    let creds = Credentials::new(sender, sender_password);
    let mailer = SmtpTransport::relay("smtp.gmail.com")?
        .credentials(creds)
        .build();

    match mailer.send(&email_body) {
        Ok(_) => {
            tracing::info!("Email sent to {} successfully!", email);
            Ok(())
        }
        Err(err) => {
            tracing::error!("Failed to send email {:#?}", err);
            Err(Box::new(err))
        }
    }
}

pub async fn send_password_reset_email(
    body: Json<UserEmail>,
    connection: Data<PgPool>,
) -> HttpResponse {
    let is_valid = body.validate();
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
    let query_span = tracing::info_span!("Sending email to user {}", ?body.email);

    let query_res = sqlx::query!(
        r#"
            SELECT * FROM users WHERE email = $1
        "#,
        body.email
    )
    .fetch_one(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    let user = match query_res {
        Ok(user) => User {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            username: user.username,
            email: user.email,
            image_url: user.profile_picture_url,
            created_at: user.created_at.to_string(),
            updated_at: user.updated_at.to_string(),
            session_id: None,
        },
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!("User with email {} not found in database", body.email);
            return HttpResponse::NotFound().json(json!({
                "message": "User not found"
            }));
        }
        Err(err) => {
            tracing::error!("Database error {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };

    let code = generate_random_ten_character_code();

    let delete_query_res = sqlx::query(
        r#"
            DELETE FROM password_verification_code WHERE user_id = $1
        "#,
    )
    .bind(user.id)
    .execute(connection.as_ref())
    .instrument(query_span.clone())
    .await;

    match delete_query_res {
        Ok(rows_affected) => tracing::info!("Number of rows deleted = {:#?}", rows_affected),
        Err(err) => {
            tracing::error!("Failed to delete previous verification codes {:#?}", err);
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    }

    let transaction_result = connection.begin().await;
    let mut transaction = match transaction_result {
        Ok(transaction) => transaction,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "Error": e.to_string()
            }));
        }
    };

    let query_res = sqlx::query(
        r#"
            INSERT INTO password_verification_code (id, user_id, expires_at, created_at, code)
            VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(Utc::now() + chrono::Duration::minutes(10))
    .bind(Utc::now())
    .bind(code.as_str())
    .execute(&mut *transaction)
    .instrument(query_span.clone())
    .await;

    match query_res {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("Failed to create verification entry {:#?}", err);
            let _ = transaction.rollback().await;
            return HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }));
        }
    };
    let send_email_res = send_email(
        build_email(code.as_str(), user.username.as_str()),
        user.email.as_str(),
    );
    match send_email_res {
        Ok(_) => {
            let res = transaction.commit().await;
            if res.is_err() {
                tracing::error!("failed to write changes to the database");
                return HttpResponse::BadRequest().json(json!({
                    "error": "failed to write changes to the database"
                }));
            }
            HttpResponse::Ok().json(json!({
                "message": "Email sent"
            }))
        }
        Err(_) => {
            let res = transaction.rollback().await;
            if res.is_err() {
                tracing::error!("failed to rollback changes in the database");
                return HttpResponse::BadRequest().json(json!({
                    "error": "failed to rollback changes in the database"
                }));
            }
            HttpResponse::BadRequest().json(json!({
                "error": "something went wrong"
            }))
        }
    }
}
