use serde::{Deserialize, Serialize};
use uuid::Uuid;
use redis::RedisError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(with = "uuid_serde")]
    pub id: Uuid,
    pub topic: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

impl Message {
    pub fn new(topic: String, payload: Vec<u8>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
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
    #[error("Redis error: {0}")]
    RedisError(#[from] RedisError),
}

mod uuid_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&uuid.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(serde::de::Error::custom)
    }
} 