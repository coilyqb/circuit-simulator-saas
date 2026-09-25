use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Circuit {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub netlist_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Component {
    pub id: i64,
    pub circuit_id: i64,
    pub component_type: String,
    pub value: String,
    pub nodes: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Connection {
    pub id: i64,
    pub circuit_id: i64,
    pub from_node: String,
    pub to_node: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCircuitRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub netlist_json: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCircuitRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub netlist_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComponentRequest {
    pub component_type: String,
    pub value: String,
    pub nodes: Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub from_node: String,
    pub to_node: String,
}

#[derive(Debug, Serialize)]
pub struct CircuitDetail {
    pub circuit: Circuit,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
}
