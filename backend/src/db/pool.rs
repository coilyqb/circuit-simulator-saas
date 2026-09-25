use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

use crate::errors::AppError;

pub async fn create_pool(database_url: &str) -> Result<MySqlPool, AppError> {
    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(AppError::from)
}
