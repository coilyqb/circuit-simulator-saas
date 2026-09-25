use chrono::Utc;
use serde_json::json;
use sqlx::MySqlPool;

use crate::{
    errors::AppError,
    models::{
        job::{AdminStats, Job},
        simulation::{
            CreateSimulationRequest, Simulation, SimulationConfig, SimulationDetail,
            SimulationResult, SimulationStatusResponse, SubmitSimulationResponse,
            UpdateSimulationConfigRequest,
        },
    },
    services::{
        circuit::CircuitService,
        job_queue::{JobQueueService, JobRequest},
        notification::NotificationService,
    },
    utils::netlist::generate_netlist,
};

#[derive(Clone)]
pub struct SimulationService {
    pool: MySqlPool,
    queue: JobQueueService,
    notifications: NotificationService,
    circuit_service: CircuitService,
}

impl SimulationService {
    pub fn new(
        pool: MySqlPool,
        queue: JobQueueService,
        notifications: NotificationService,
        circuit_service: CircuitService,
    ) -> Self {
        Self {
            pool,
            queue,
            notifications,
            circuit_service,
        }
    }

    pub async fn list_simulations(&self, user_id: i64) -> Result<Vec<Simulation>, AppError> {
        sqlx::query_as::<_, Simulation>(
            r#"
            SELECT id, circuit_id, user_id, name, status, created_at, updated_at
            FROM simulations
            WHERE user_id = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn create_simulation(
        &self,
        user_id: i64,
        request: CreateSimulationRequest,
    ) -> Result<SimulationDetail, AppError> {
        self.circuit_service
            .ensure_circuit_ownership(user_id, request.circuit_id)
            .await?;
        validate_analysis_type(&request.analysis_type)?;

        let mut transaction = self.pool.begin().await?;

        let simulation_result = sqlx::query(
            r#"
            INSERT INTO simulations (circuit_id, user_id, name, status)
            VALUES (?, ?, ?, 'pending')
            "#,
        )
        .bind(request.circuit_id)
        .bind(user_id)
        .bind(request.name)
        .execute(&mut *transaction)
        .await?;

        let simulation_id = simulation_result.last_insert_id() as i64;

        sqlx::query(
            r#"
            INSERT INTO simulation_configs (simulation_id, analysis_type, parameters_json)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(simulation_id)
        .bind(request.analysis_type)
        .bind(if request.parameters_json.is_null() {
            json!({})
        } else {
            request.parameters_json
        })
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        self.get_simulation_detail(user_id, simulation_id).await
    }

    pub async fn get_simulation_detail(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<SimulationDetail, AppError> {
        let simulation = self.get_simulation(user_id, simulation_id).await?;
        let config = self.get_simulation_config(simulation_id).await?;
        let result = self.get_result(simulation_id).await?;
        let latest_job = self.get_latest_job(simulation_id).await?;

        Ok(SimulationDetail {
            simulation,
            config,
            result,
            latest_job,
        })
    }

    pub async fn update_config(
        &self,
        user_id: i64,
        simulation_id: i64,
        request: UpdateSimulationConfigRequest,
    ) -> Result<SimulationConfig, AppError> {
        self.get_simulation(user_id, simulation_id).await?;
        let current = self
            .get_simulation_config(simulation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("simulation config not found".to_owned()))?;

        let analysis_type = request.analysis_type.unwrap_or(current.analysis_type);
        validate_analysis_type(&analysis_type)?;
        let parameters_json = request.parameters_json.unwrap_or(current.parameters_json);

        sqlx::query(
            r#"
            UPDATE simulation_configs
            SET analysis_type = ?, parameters_json = ?
            WHERE simulation_id = ?
            "#,
        )
        .bind(analysis_type)
        .bind(parameters_json)
        .bind(simulation_id)
        .execute(&self.pool)
        .await?;

        self.get_simulation_config(simulation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("simulation config not found".to_owned()))
    }

    pub async fn submit_simulation(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<SubmitSimulationResponse, AppError> {
        let simulation = self.get_simulation(user_id, simulation_id).await?;
        let config = self
            .get_simulation_config(simulation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("simulation config not found".to_owned()))?;
        let circuit_detail = self
            .circuit_service
            .get_circuit_detail(user_id, simulation.circuit_id)
            .await?;
        let netlist = generate_netlist(
            &circuit_detail.circuit,
            &circuit_detail.components,
            &circuit_detail.connections,
            Some(&config),
        )?;

        let job_result = sqlx::query(
            r#"
            INSERT INTO jobs (simulation_id, status)
            VALUES (?, 'pending')
            "#,
        )
        .bind(simulation_id)
        .execute(&self.pool)
        .await?;

        let job_id = job_result.last_insert_id() as i64;

        sqlx::query(
            r#"
            UPDATE simulations
            SET status = 'pending', updated_at = UTC_TIMESTAMP()
            WHERE id = ?
            "#,
        )
        .bind(simulation_id)
        .execute(&self.pool)
        .await?;

        self.queue
            .enqueue(JobRequest {
                job_id,
                simulation_id,
                user_id,
                netlist: netlist.clone(),
                analysis_type: config.analysis_type.clone(),
                parameters_json: config.parameters_json.clone(),
            })
            .await?;

        self.notifications
            .publish(
                simulation_id,
                &json!({
                    "simulation_id": simulation_id,
                    "job_id": job_id,
                    "status": "pending",
                    "timestamp": Utc::now(),
                }),
            )
            .await?;

        Ok(SubmitSimulationResponse {
            job_id,
            simulation_id,
            status: "pending".to_owned(),
            queued_at: Utc::now(),
            netlist,
        })
    }

    pub async fn get_results(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<Option<SimulationResult>, AppError> {
        self.get_simulation(user_id, simulation_id).await?;
        self.get_result(simulation_id).await
    }

    pub async fn get_status(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<SimulationStatusResponse, AppError> {
        let simulation = self.get_simulation(user_id, simulation_id).await?;
        let latest_job = self.get_latest_job(simulation_id).await?;

        Ok(SimulationStatusResponse {
            simulation_id,
            status: latest_job
                .as_ref()
                .map(|job| job.status.clone())
                .unwrap_or_else(|| simulation.status.clone()),
            job_id: latest_job.as_ref().map(|job| job.id),
            worker_id: latest_job.and_then(|job| job.worker_id),
            updated_at: simulation.updated_at,
        })
    }

    pub async fn list_jobs(&self) -> Result<Vec<Job>, AppError> {
        sqlx::query_as::<_, Job>(
            r#"
            SELECT id, simulation_id, status, worker_id, created_at, completed_at
            FROM jobs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn get_admin_stats(&self) -> Result<AdminStats, AppError> {
        let users = count_rows(&self.pool, "users").await?;
        let circuits = count_rows(&self.pool, "circuits").await?;
        let simulations = count_rows(&self.pool, "simulations").await?;
        let jobs = count_rows(&self.pool, "jobs").await?;
        let queued_jobs = count_jobs_by_status(&self.pool, "pending").await?;
        let running_jobs = count_jobs_by_status(&self.pool, "running").await?;
        let completed_jobs = count_jobs_by_status(&self.pool, "completed").await?;
        let failed_jobs = count_jobs_by_status(&self.pool, "failed").await?;

        Ok(AdminStats {
            users,
            circuits,
            simulations,
            jobs,
            queued_jobs,
            running_jobs,
            completed_jobs,
            failed_jobs,
        })
    }

    pub async fn simulation_belongs_to_user(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM simulations WHERE id = ? AND user_id = ?",
        )
        .bind(simulation_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    async fn get_simulation(
        &self,
        user_id: i64,
        simulation_id: i64,
    ) -> Result<Simulation, AppError> {
        sqlx::query_as::<_, Simulation>(
            r#"
            SELECT id, circuit_id, user_id, name, status, created_at, updated_at
            FROM simulations
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(simulation_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("simulation not found".to_owned()))
    }

    async fn get_simulation_config(
        &self,
        simulation_id: i64,
    ) -> Result<Option<SimulationConfig>, AppError> {
        sqlx::query_as::<_, SimulationConfig>(
            r#"
            SELECT id, simulation_id, analysis_type, parameters_json, created_at
            FROM simulation_configs
            WHERE simulation_id = ?
            "#,
        )
        .bind(simulation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)
    }

    async fn get_result(&self, simulation_id: i64) -> Result<Option<SimulationResult>, AppError> {
        sqlx::query_as::<_, SimulationResult>(
            r#"
            SELECT id, simulation_id, waveform_data_json, measurements_json, created_at
            FROM results
            WHERE simulation_id = ?
            "#,
        )
        .bind(simulation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)
    }

    async fn get_latest_job(&self, simulation_id: i64) -> Result<Option<Job>, AppError> {
        sqlx::query_as::<_, Job>(
            r#"
            SELECT id, simulation_id, status, worker_id, created_at, completed_at
            FROM jobs
            WHERE simulation_id = ?
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(simulation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)
    }
}

fn validate_analysis_type(analysis_type: &str) -> Result<(), AppError> {
    let is_supported = matches!(analysis_type, "dc" | "transient" | "ac" | "sweep");

    if !is_supported {
        return Err(AppError::Validation(format!(
            "unsupported analysis type `{analysis_type}`"
        )));
    }

    Ok(())
}

async fn count_rows(pool: &MySqlPool, table: &str) -> Result<i64, AppError> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(&query)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
}

async fn count_jobs_by_status(pool: &MySqlPool, status: &str) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM jobs WHERE status = ?")
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
}
