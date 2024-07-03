mod test_startup;
use reqwest;
use serde::Deserialize;
use test_startup::*;


#[derive(Deserialize)]
struct ResponseMessage {
    message: String,
}

#[actix_rt::test]
async fn check_server_health() {
    let app = spawn_app().await;
    let client: reqwest::Client = reqwest::Client::new();

    let res = client
        .get(format!("{}", app.address.as_str()).as_str())
        .send()
        .await
        .expect("Failed to execute request");

    assert!(res.status().is_success());
    let body = res
        .json::<ResponseMessage>()
        .await
        .expect("Failed to parse the response body");
    assert_eq!(body.message.as_str(), "Hello From Actix Server!!");
}
