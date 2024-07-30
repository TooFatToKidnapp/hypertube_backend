use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use serde::Deserialize;
use sqlx::PgPool;

use super::{MovieQuality, Source};

#[derive(Deserialize)]
pub struct StreamInfo {
    pub movie_id: u32,
    pub source: Source,
    pub quality: MovieQuality,
}

pub async fn stream_video_content(
    _connection: Data<PgPool>,
    _info: Path<StreamInfo>,
) -> HttpResponse {
    todo!()
}
