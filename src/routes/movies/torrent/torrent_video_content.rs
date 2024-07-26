use reqwest::Client;
use serde_json::Value;
// https://github.com/ikatson/rqbit

pub async fn start_torrent_download(
    magnet_link: String,
) -> Result<i64, Box<dyn std::error::Error>> {
    let client = Client::new();

    let start_torrent_res = client
        .post("http://127.0.0.1/3030/torrents")
        .json(magnet_link.as_str())
        .send()
        .await?
        .json::<Value>()
        .await?;

    let id = if start_torrent_res["id"].as_i64().is_none() {
        return Err(Box::<dyn std::error::Error>::from(
            "Missing torrent id".to_string(),
        ));
    } else {
        start_torrent_res["id"].as_i64().unwrap()
    };
    Ok(id)
}
