pub mod pool;

use std::path::Path;

use sqlx::{MySqlPool, migrate::Migrator};

use crate::errors::AppError;

pub async fn run_migrations(pool: &MySqlPool) -> Result<(), AppError> {
    let migrator = Migrator::new(Path::new("./src/db/migrations"))
        .await
        .map_err(|error| AppError::Internal(format!("failed to load migrations: {error}")))?;

    migrator.run(pool).await?;
    Ok(())
}
