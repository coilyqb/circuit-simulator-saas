use serde_json::json;
use sqlx::MySqlPool;

use crate::{
    errors::AppError,
    models::circuit::{
        Circuit, CircuitDetail, Component, Connection, CreateCircuitRequest, CreateComponentRequest,
        CreateConnectionRequest, UpdateCircuitRequest,
    },
};

#[derive(Clone)]
pub struct CircuitService {
    pool: MySqlPool,
}

impl CircuitService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_circuits(&self, user_id: i64) -> Result<Vec<Circuit>, AppError> {
        sqlx::query_as::<_, Circuit>(
            r#"
            SELECT id, user_id, name, description, netlist_json, created_at, updated_at
            FROM circuits
            WHERE user_id = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn create_circuit(
        &self,
        user_id: i64,
        request: CreateCircuitRequest,
    ) -> Result<Circuit, AppError> {
        if request.name.trim().is_empty() {
            return Err(AppError::Validation("circuit name is required".to_owned()));
        }

        let netlist_json = if request.netlist_json.is_null() {
            json!({})
        } else {
            request.netlist_json
        };

        let result = sqlx::query(
            r#"
            INSERT INTO circuits (user_id, name, description, netlist_json)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(request.name)
        .bind(request.description)
        .bind(netlist_json)
        .execute(&self.pool)
        .await?;

        self.get_circuit(user_id, result.last_insert_id() as i64).await
    }

    pub async fn get_circuit_detail(
        &self,
        user_id: i64,
        circuit_id: i64,
    ) -> Result<CircuitDetail, AppError> {
        let circuit = self.get_circuit(user_id, circuit_id).await?;
        let components = self.list_components(circuit_id).await?;
        let connections = self.list_connections(circuit_id).await?;

        Ok(CircuitDetail {
            circuit,
            components,
            connections,
        })
    }

    pub async fn update_circuit(
        &self,
        user_id: i64,
        circuit_id: i64,
        request: UpdateCircuitRequest,
    ) -> Result<Circuit, AppError> {
        let current = self.get_circuit(user_id, circuit_id).await?;
        let name = request.name.unwrap_or(current.name);
        let description = request.description.or(current.description);
        let netlist_json = request.netlist_json.unwrap_or(current.netlist_json);

        sqlx::query(
            r#"
            UPDATE circuits
            SET name = ?, description = ?, netlist_json = ?, updated_at = UTC_TIMESTAMP()
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(netlist_json)
        .bind(circuit_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        self.get_circuit(user_id, circuit_id).await
    }

    pub async fn delete_circuit(&self, user_id: i64, circuit_id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM circuits WHERE id = ? AND user_id = ?")
            .bind(circuit_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("circuit not found".to_owned()));
        }

        Ok(())
    }

    pub async fn add_component(
        &self,
        user_id: i64,
        circuit_id: i64,
        request: CreateComponentRequest,
    ) -> Result<Component, AppError> {
        self.ensure_circuit_ownership(user_id, circuit_id).await?;
        validate_component_type(&request.component_type)?;

        let result = sqlx::query(
            r#"
            INSERT INTO components (circuit_id, component_type, value, nodes)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(circuit_id)
        .bind(request.component_type)
        .bind(request.value)
        .bind(request.nodes)
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, Component>(
            r#"
            SELECT id, circuit_id, component_type, value, nodes, created_at
            FROM components
            WHERE id = ?
            "#,
        )
        .bind(result.last_insert_id() as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn delete_component(
        &self,
        user_id: i64,
        circuit_id: i64,
        component_id: i64,
    ) -> Result<(), AppError> {
        self.ensure_circuit_ownership(user_id, circuit_id).await?;

        let result = sqlx::query("DELETE FROM components WHERE id = ? AND circuit_id = ?")
            .bind(component_id)
            .bind(circuit_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("component not found".to_owned()));
        }

        Ok(())
    }

    pub async fn add_connection(
        &self,
        user_id: i64,
        circuit_id: i64,
        request: CreateConnectionRequest,
    ) -> Result<Connection, AppError> {
        self.ensure_circuit_ownership(user_id, circuit_id).await?;

        let result = sqlx::query(
            r#"
            INSERT INTO connections (circuit_id, from_node, to_node)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(circuit_id)
        .bind(request.from_node)
        .bind(request.to_node)
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, Connection>(
            r#"
            SELECT id, circuit_id, from_node, to_node, created_at
            FROM connections
            WHERE id = ?
            "#,
        )
        .bind(result.last_insert_id() as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn delete_connection(
        &self,
        user_id: i64,
        circuit_id: i64,
        connection_id: i64,
    ) -> Result<(), AppError> {
        self.ensure_circuit_ownership(user_id, circuit_id).await?;

        let result = sqlx::query("DELETE FROM connections WHERE id = ? AND circuit_id = ?")
            .bind(connection_id)
            .bind(circuit_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("connection not found".to_owned()));
        }

        Ok(())
    }

    pub async fn get_circuit(&self, user_id: i64, circuit_id: i64) -> Result<Circuit, AppError> {
        sqlx::query_as::<_, Circuit>(
            r#"
            SELECT id, user_id, name, description, netlist_json, created_at, updated_at
            FROM circuits
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(circuit_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("circuit not found".to_owned()))
    }

    pub async fn list_components(&self, circuit_id: i64) -> Result<Vec<Component>, AppError> {
        sqlx::query_as::<_, Component>(
            r#"
            SELECT id, circuit_id, component_type, value, nodes, created_at
            FROM components
            WHERE circuit_id = ?
            ORDER BY id ASC
            "#,
        )
        .bind(circuit_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn list_connections(&self, circuit_id: i64) -> Result<Vec<Connection>, AppError> {
        sqlx::query_as::<_, Connection>(
            r#"
            SELECT id, circuit_id, from_node, to_node, created_at
            FROM connections
            WHERE circuit_id = ?
            ORDER BY id ASC
            "#,
        )
        .bind(circuit_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn ensure_circuit_ownership(
        &self,
        user_id: i64,
        circuit_id: i64,
    ) -> Result<(), AppError> {
        self.get_circuit(user_id, circuit_id).await.map(|_| ())
    }
}

fn validate_component_type(component_type: &str) -> Result<(), AppError> {
    let is_supported = matches!(
        component_type,
        "R" | "L" | "C" | "BJT" | "CMOS" | "Diode" | "VSource" | "ISource"
    );

    if !is_supported {
        return Err(AppError::Validation(format!(
            "unsupported component type `{component_type}`"
        )));
    }

    Ok(())
}
