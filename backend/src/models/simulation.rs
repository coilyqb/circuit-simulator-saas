use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

use crate::models::job::Job;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Simulation {
    pub id: i64,
    pub circuit_id: i64,
    pub user_id: i64,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SimulationConfig {
    pub id: i64,
    pub simulation_id: i64,
    pub analysis_type: String,
    pub parameters_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SimulationResult {
    pub id: i64,
    pub simulation_id: i64,
    pub waveform_data_json: Value,
    pub measurements_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSimulationRequest {
    pub circuit_id: i64,
    pub name: String,
    pub analysis_type: String,
    pub parameters_json: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSimulationConfigRequest {
    pub analysis_type: Option<String>,
    pub parameters_json: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct SimulationDetail {
    pub simulation: Simulation,
    pub config: Option<SimulationConfig>,
    pub result: Option<SimulationResult>,
    pub latest_job: Option<Job>,
}

#[derive(Debug, Serialize)]
pub struct SimulationStatusResponse {
    pub simulation_id: i64,
    pub status: String,
    pub job_id: Option<i64>,
    pub worker_id: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SubmitSimulationResponse {
    pub job_id: i64,
    pub simulation_id: i64,
    pub status: String,
    pub queued_at: DateTime<Utc>,
    pub netlist: String,
}
