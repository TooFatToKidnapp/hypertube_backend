use reqwest::Client;
use serde_json::Value;
use std::env;
pub struct RqbitWrapper {
    pub origin: String,
    pub download_path: String,
}

pub struct SubInfo {
    pub language: String,
    pub path: String,
}

pub struct FileInfo {
    pub id: String,
    pub path: String,
    pub available_subs: Option<Vec<SubInfo>>,
    pub file_type: String,
}

impl FileInfo {
    fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        available_subs: Option<Vec<SubInfo>>,
        file_type: impl Into<String>,
    ) -> Self {
        FileInfo {
            id: id.into(),
            path: path.into(),
            available_subs,
            file_type: file_type.into(),
        }
    }
}

impl Default for RqbitWrapper {
    fn default() -> Self {
        let current_working_dir = match env::current_dir() {
            Ok(dir) => dir.display().to_string(),
            Err(_err) => "/tmp/Download".to_string(),
        };
        RqbitWrapper::new("http://127.0.0.1:3030", current_working_dir)
    }
}

fn is_video_file(file_name: &str) -> bool {
    let video_extensions = vec![".mp4", ".mkv", ".flv", ".avi", ".mov", ".wmv"];
    video_extensions.iter().any(|ext| file_name.ends_with(ext))
}

impl RqbitWrapper {
    pub fn new(origin: impl Into<String>, download_path: impl Into<String>) -> Self {
        RqbitWrapper {
            origin: origin.into(),
            download_path: download_path.into(),
        }
    }

    pub async fn download_torrent(
        &self,
        magnet: impl Into<String>,
        output_folder: Option<impl Into<String>>,
    ) -> Result<FileInfo, String> {
        let client = Client::new();
        let response = match client
            .post(format!("{}/torrents", self.origin.as_str()))
            .json(&magnet.into())
            .send()
            .await
        {
            Ok(res) => match res.json::<Value>().await {
                Ok(body) => body,
                Err(err) => {
                    return Err("Error: Failed to get request body".to_string());
                }
            },
            Err(err) => {
                return Err("Error: Failed to request torrent client".to_string());
            }
        };
        let torrent_id = match response["id"].as_number() {
            Some(id) => id.to_string(),
            None => return Err("Error: Not torrent id in response body".to_string()),
        };
        let torrent_path = {
            if output_folder.is_none() {
                let torrent_dir_name = response["name"].as_str();
                if torrent_dir_name.is_none() {
                    return Err("Error: Missing torrent name form response body".to_string());
                }
                format!("{}/{}", self.download_path, torrent_dir_name.unwrap())
            } else {
                output_folder.unwrap().into()
            }
        };
        let (torrent_subs, torrent_file_type) = {
            let torrent_files_arr = match response["files"].as_array() {
                Some(fiels) => fiels,
                None => return Err("Error: Found no files in response".to_string()),
            };
            let mut torrent_type = String::new();
            let mut torrent_sub_arr = Vec::<SubInfo>::new();
            for file in torrent_files_arr.iter() {
                if !file["name"].is_string() {
                    continue;
                }
                let file_as_str = file["name"].as_str().unwrap();
                if file_as_str.ends_with(".srt") {
                    let path = file_as_str.to_string();
                    let language = match file_as_str.strip_prefix("Subs/") {
                        Some(file) => file.strip_suffix(".srt").unwrap(),
                        None => file_as_str.strip_suffix(".srt").unwrap(),
                    };
                    torrent_sub_arr.push(SubInfo {
                        path,
                        language: language.to_string(),
                    });
                } else if is_video_file(file_as_str) {
                    let tab = file_as_str.trim().split(".");
                    tab.last().unwrap().clone_into(&mut torrent_type);
                }
            }
            if torrent_sub_arr.is_empty() {
                (None::<Vec<SubInfo>>, torrent_type)
            } else {
                (Some(torrent_sub_arr), torrent_type)
            }
        };
        let torrent = FileInfo::new(torrent_id, torrent_path, torrent_subs, torrent_file_type);
        Ok(torrent)
    }
}
