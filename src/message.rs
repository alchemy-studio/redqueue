use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub topic: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

impl Message {
    pub fn new(topic: String, payload: Vec<u8>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: Uuid::new_v4(),
            topic,
            payload,
            timestamp,
            metadata: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AckMessage {
    Data(Message),
    Ack(Uuid),
}

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Topic not found")]
    TopicNotFound,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
} 