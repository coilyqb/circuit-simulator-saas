use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use tokio::sync::{RwLock, broadcast};

use crate::errors::AppError;

#[derive(Clone, Default)]
pub struct NotificationService {
    channels: Arc<RwLock<HashMap<i64, broadcast::Sender<String>>>>,
}

impl NotificationService {
    pub async fn subscribe(&self, simulation_id: i64) -> broadcast::Receiver<String> {
        let mut channels = self.channels.write().await;
        channels
            .entry(simulation_id)
            .or_insert_with(|| broadcast::channel(100).0)
            .subscribe()
    }

    pub async fn publish<T>(&self, simulation_id: i64, payload: &T) -> Result<(), AppError>
    where
        T: Serialize,
    {
        let message = serde_json::to_string(payload)
            .map_err(|error| AppError::Internal(format!("failed to serialize message: {error}")))?;
        self.publish_text(simulation_id, message).await;
        Ok(())
    }

    pub async fn publish_text(&self, simulation_id: i64, payload: String) {
        let mut channels = self.channels.write().await;
        let sender = channels
            .entry(simulation_id)
            .or_insert_with(|| broadcast::channel(100).0);
        let _ = sender.send(payload);
    }
}
