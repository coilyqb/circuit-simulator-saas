use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use chrono::Utc;
use serde_json::json;
use sqlx::MySqlPool;
use tokio::{
    sync::{Mutex, mpsc},
    time::{Duration, sleep},
};
use tracing::{error, info};

use crate::{errors::AppError, services::notification::NotificationService};

#[derive(Debug, Clone)]
pub struct JobRequest {
    pub job_id: i64,
    pub simulation_id: i64,
    pub user_id: i64,
    pub netlist: String,
    pub analysis_type: String,
    pub parameters_json: serde_json::Value,
}

#[derive(Clone)]
pub struct JobQueueService {
    sender: mpsc::Sender<JobRequest>,
    queue_depth: Arc<AtomicUsize>,
}

impl JobQueueService {
    pub fn new(buffer: usize) -> (Self, mpsc::Receiver<JobRequest>) {
        let (sender, receiver) = mpsc::channel(buffer);
        (
            Self {
                sender,
                queue_depth: Arc::new(AtomicUsize::new(0)),
            },
            receiver,
        )
    }

    pub async fn enqueue(&self, job: JobRequest) -> Result<(), AppError> {
        self.queue_depth.fetch_add(1, Ordering::SeqCst);
        if let Err(error) = self.sender.send(job).await {
            self.queue_depth.fetch_sub(1, Ordering::SeqCst);
            return Err(AppError::Internal(format!("failed to enqueue job: {error}")));
        }
        Ok(())
    }

    pub fn queue_depth(&self) -> usize {
        self.queue_depth.load(Ordering::SeqCst)
    }

    pub fn start_workers(
        receiver: mpsc::Receiver<JobRequest>,
        pool: MySqlPool,
        notifications: NotificationService,
        worker_count: usize,
        simulation_delay_ms: u64,
        queue_depth: Arc<AtomicUsize>,
    ) {
        let receiver = Arc::new(Mutex::new(receiver));

        for worker_number in 0..worker_count.max(1) {
            let receiver = Arc::clone(&receiver);
            let pool = pool.clone();
            let notifications = notifications.clone();
            let queue_depth = Arc::clone(&queue_depth);
            let worker_id = format!("worker-{}", worker_number + 1);

            tokio::spawn(async move {
                loop {
                    let message = {
                        let mut receiver = receiver.lock().await;
                        receiver.recv().await
                    };

                    let Some(job) = message else {
                        break;
                    };

                    queue_depth.fetch_sub(1, Ordering::SeqCst);

                    if let Err(error) = process_job(
                        &pool,
                        &notifications,
                        &worker_id,
                        simulation_delay_ms,
                        job,
                    )
                    .await
                    {
                        error!(worker_id, ?error, "job processing failed");
                    }
                }
            });
        }
    }

    pub fn depth_handle(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.queue_depth)
    }
}

async fn process_job(
    pool: &MySqlPool,
    notifications: &NotificationService,
    worker_id: &str,
    simulation_delay_ms: u64,
    job: JobRequest,
) -> Result<(), AppError> {
    info!(job_id = job.job_id, simulation_id = job.simulation_id, "processing job");

    sqlx::query(
        r#"
        UPDATE jobs
        SET status = 'running', worker_id = ?
        WHERE id = ?
        "#,
    )
    .bind(worker_id)
    .bind(job.job_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE simulations
        SET status = 'running', updated_at = UTC_TIMESTAMP()
        WHERE id = ?
        "#,
    )
    .bind(job.simulation_id)
    .execute(pool)
    .await?;

    notifications
        .publish(
            job.simulation_id,
            &json!({
                "simulation_id": job.simulation_id,
                "job_id": job.job_id,
                "status": "running",
                "worker_id": worker_id,
                "timestamp": Utc::now(),
            }),
        )
        .await?;

    sleep(Duration::from_millis(simulation_delay_ms)).await;

    let waveform = json!({
        "analysis_type": job.analysis_type,
        "parameters": job.parameters_json,
        "netlist": job.netlist,
        "points": [
            { "time": 0.0, "value": 0.0 },
            { "time": 1.0, "value": 1.8 },
            { "time": 2.0, "value": 0.9 }
        ]
    });
    let measurements = json!({
        "status": "placeholder_result",
        "max_voltage": 1.8,
        "min_voltage": 0.0,
        "processed_by": worker_id,
        "user_id": job.user_id
    });

    sqlx::query(
        r#"
        INSERT INTO results (simulation_id, waveform_data_json, measurements_json)
        VALUES (?, ?, ?)
        ON DUPLICATE KEY UPDATE
            waveform_data_json = VALUES(waveform_data_json),
            measurements_json = VALUES(measurements_json),
            created_at = UTC_TIMESTAMP()
        "#,
    )
    .bind(job.simulation_id)
    .bind(waveform)
    .bind(measurements)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE jobs
        SET status = 'completed', completed_at = UTC_TIMESTAMP(), worker_id = ?
        WHERE id = ?
        "#,
    )
    .bind(worker_id)
    .bind(job.job_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE simulations
        SET status = 'completed', updated_at = UTC_TIMESTAMP()
        WHERE id = ?
        "#,
    )
    .bind(job.simulation_id)
    .execute(pool)
    .await?;

    notifications
        .publish(
            job.simulation_id,
            &json!({
                "simulation_id": job.simulation_id,
                "job_id": job.job_id,
                "status": "completed",
                "worker_id": worker_id,
                "timestamp": Utc::now(),
            }),
        )
        .await?;

    Ok(())
}
