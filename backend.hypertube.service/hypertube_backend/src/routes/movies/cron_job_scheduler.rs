use super::torrent::RqbitWrapper;
use super::Source;
use actix_web::web::Data;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

pub struct CronJobScheduler {
    scheduled_jobs: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl Default for CronJobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CronJobScheduler {
    pub fn new() -> Self {
        Self {
            scheduled_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn build_job_id(movie_id: i32, source: Source) -> String {
        format!("{}_{}", movie_id, source)
    }
}

async fn delete_movie_handler(
    job_list: Data<CronJobScheduler>,
    job_id: String,
    connection: Data<PgPool>,
) {
    let interval = time::interval(Duration::from_secs(2_628_000)); // one month in seconds
    let mut interval = interval;

    interval.tick().await;
    interval.tick().await;

    {
        let mut job_list = job_list.scheduled_jobs.lock().unwrap();
        job_list.remove(&job_id);
    }

    let movie_identifier: Vec<String> = job_id.split('_').map(|s| s.to_string()).collect();
    // let src: Source = movie_identifier[1];
    let (torrent_id, torrent_path) = match sqlx::query(
        r#"
            DELETE FROM movie_torrent WHERE movie_id = $1 AND movie_source = $2
            RETURNING *
        "#,
    )
    .bind(movie_identifier[0].parse::<i32>().unwrap())
    .bind::<Source>(movie_identifier[1].clone().into())
    .fetch_one(connection.as_ref())
    .await
    {
        Ok(row) => {
            tracing::info!("Deleted movie record form database successfully {}", job_id);
            (row.get("torrent_id"), row.get("movie_path"))
        }
        Err(err) => {
            tracing::error!("Database Error Failed to delete Movie {err}");
            return;
        }
    };
    let torrent_client = RqbitWrapper::default();
    match torrent_client
        .delete_torrent(torrent_id, torrent_path)
        .await
    {
        Ok(_) => tracing::info!("Deleted Movie from file system"),
        Err(err) => tracing::error!("Cant delete Movie form file system: {}", err),
    };
}

pub async fn schedule_handler<'a>(
    job_list: &'a Data<CronJobScheduler>,
    job_id: String,
    connection: &'a Data<PgPool>,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let job_handle = tokio::spawn(delete_movie_handler(
        job_list.clone(),
        job_id.clone(),
        connection.clone(),
    ));
    {
        let mut jobs = job_list.scheduled_jobs.lock()?;
        jobs.insert(job_id, job_handle);
    }
    tracing::debug!(
        "Movie scheduled for deletion in : {:#?}",
        Duration::from_secs(60)
    );

    Ok(())
}

pub async fn cancel_job<'a>(
    job_list: &'a mut Data<CronJobScheduler>,
    job_id: String,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let mut jobs = job_list.scheduled_jobs.lock()?;
    if let Some(job_handle) = jobs.remove(&job_id) {
        tracing::debug!("REMOVED JOB HANDLER FOR JOB: {}", job_id);
        job_handle.abort();
    }

    Ok(())
}
