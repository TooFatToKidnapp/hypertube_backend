use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use sqlx::PgPool;

use super::{MovieQuality, Source};

struct StreamInfo {
    movie_id: u32,
    source: Source,
    quality: MovieQuality,
}

pub async fn stream_video_content(
    connection: Data<PgPool>,
    info: Path<StreamInfo>,
) -> HttpResponse {
    todo!()
}
