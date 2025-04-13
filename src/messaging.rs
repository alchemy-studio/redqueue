use crate::message::{AckMessage, Message, MessageError};
use async_trait::async_trait;
use futures::Stream;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
    time,
};

type MessageStream = Pin<Box<dyn Stream<Item = Message> + Send>>;
type MessageFilter = Box<dyn Fn(&Message) -> bool + Send + Sync>;

pub struct RedQueue {
    topics: Arc<RwLock<HashMap<String, Vec<Sender<Message>>>>>,
    redis: ConnectionManager,
    cleanup_interval: Duration,
}

impl RedQueue {
    pub async fn new(redis_url: &str, cleanup_interval: Duration) -> Result<Self, MessageError> {
        let client = Client::open(redis_url)?;
        let redis = ConnectionManager::new(client).await?;

        Ok(Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            redis,
            cleanup_interval,
        })
    }

    pub async fn publish(&self, topic: &str, message: Message) -> Result<(), MessageError> {
        // Save message to Redis
        self.save_to_redis(topic, &message).await?;

        // Get topic subscribers
        let topics = self.topics.read().await;
        if let Some(subscribers) = topics.get(topic) {
            // Send message to all subscribers
            for subscriber in subscribers {
                if let Err(_) = subscriber.send(message.clone()).await {
                    // Handle failed sends (e.g., subscriber disconnected)
                }
            }
        }

        Ok(())
    }

    pub async fn subscribe(&self, topic: &str) -> Result<MessageStream, MessageError> {
        let (tx, rx) = mpsc::channel(100);
        
        // Add subscriber to topic
        let mut topics = self.topics.write().await;
        topics.entry(topic.to_string()).or_insert_with(Vec::new).push(tx);

        // Convert receiver to stream
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    pub async fn subscribe_with_filter<F>(
        &self,
        topic: &str,
        filter: F,
    ) -> Result<MessageStream, MessageError>
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel(100);
        let filter = Arc::new(filter);

        // Add subscriber to topic
        let mut topics = self.topics.write().await;
        topics.entry(topic.to_string()).or_insert_with(Vec::new).push(tx);

        // Convert receiver to stream with filter
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
            .filter(move |msg| {
                let filter = filter.clone();
                async move { filter(msg) }
            });

        Ok(Box::pin(stream))
    }

    async fn save_to_redis(&self, topic: &str, message: &Message) -> Result<(), MessageError> {
        let mut conn = self.redis.clone();
        
        // Store message in Redis
        let message_key = format!("message:{}:{}", topic, message.id);
        let message_json = serde_json::to_string(message)?;
        
        // Store message data
        conn.set(&message_key, &message_json).await?;
        
        // Add message ID to topic's message list
        let topic_key = format!("topic:{}:messages", topic);
        conn.lpush(&topic_key, message.id.to_string()).await?;

        Ok(())
    }

    pub async fn get_messages(&self, topic: &str, count: isize) -> Result<Vec<Message>, MessageError> {
        let mut conn = self.redis.clone();
        let topic_key = format!("topic:{}:messages", topic);
        
        // Get message IDs from Redis
        let message_ids: Vec<String> = conn.lrange(&topic_key, 0, count - 1).await?;
        
        let mut messages = Vec::new();
        for id in message_ids {
            let message_key = format!("message:{}:{}", topic, id);
            if let Ok(message_json) = conn.get::<_, String>(&message_key).await {
                if let Ok(message) = serde_json::from_str::<Message>(&message_json) {
                    messages.push(message);
                }
            }
        }
        
        Ok(messages)
    }

    pub async fn start_cleanup(&self) {
        let topics = self.topics.clone();
        let redis = self.redis.clone();
        let cleanup_interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let topics = topics.read().await;
                for (topic, subscribers) in topics.iter() {
                    if subscribers.is_empty() {
                        // Clean up topic in Redis
                        let mut conn = redis.clone();
                        let topic_key = format!("topic:{}:messages", topic);
                        if let Err(e) = conn.del(&topic_key).await {
                            log::error!("Failed to clean up topic {}: {}", topic, e);
                        }
                    }
                }
            }
        });
    }
} 