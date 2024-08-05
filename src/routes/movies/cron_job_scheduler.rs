use actix_web::web::Data;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

pub struct CronJobScheduler {
    scheduled_jobs: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl CronJobScheduler {
    pub fn new() -> Self {
        Self {
            scheduled_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

async fn delete_movie_handler(
    job_list: Data<CronJobScheduler>,
    job_id: String,
    connection: PgPool,
) {
    let interval = time::interval(Duration::from_secs(30 * 86400));
    let mut interval = interval;

    interval.tick().await;
    interval.tick().await;

    let mut job_list = job_list.scheduled_jobs.lock().unwrap();
    job_list.remove(&job_id);
}

pub async fn schedule_handler(
    job_list: Data<CronJobScheduler>,
    job_id: String,
    connection: &PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let job_handle = tokio::spawn(delete_movie_handler(
        job_list.clone(),
        job_id.clone(),
        connection.clone(),
    ));
    let jobs_ref = &job_list.scheduled_jobs;
    let mut jobs = match jobs_ref.lock() {
      Ok(jobs) => jobs,
      Err(err) => return Err(Box::new(err))
    };
    jobs.insert(job_id, job_handle);

    Ok(())
}

pub async fn cancel_job(job_list: Data<CronJobScheduler>, job_id: String) {}
