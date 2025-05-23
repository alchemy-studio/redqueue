use crate::message::{Message, MessageError};
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
use tokio_stream::StreamExt as TokioStreamExt;
use futures::Future;

type MessageStream = Pin<Box<dyn Stream<Item = Message> + Send>>;
type AutoAckStream = Pin<Box<dyn Stream<Item = Result<(), MessageError>> + Send>>;

#[derive(Clone)]
pub struct RedQueue {
    topics: Arc<RwLock<HashMap<String, Vec<Sender<Message>>>>>,
    redis: ConnectionManager,
    cleanup_interval: Duration,
}

impl RedQueue {
    pub async fn new(redis_url: &str, cleanup_interval: Duration) -> Result<Self, MessageError> {
        let client = Client::open(redis_url).map_err(MessageError::RedisError)?;
        let redis = ConnectionManager::new(client).await.map_err(MessageError::RedisError)?;

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
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let stream = TokioStreamExt::filter(stream, move |msg| filter(msg));

        Ok(Box::pin(stream))
    }

    pub async fn auto_ack_subscribe<F, Fut>(
        &self,
        topic: String,
        process_message: F,
    ) -> Result<AutoAckStream, MessageError>
    where
        F: Fn(Message) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<(), MessageError>> + Send + 'static,
    {
        let subscriber = self.subscribe(&topic).await?;
        let queue = self.clone();

        // Create a stream that processes messages and automatically removes them after successful processing
        let auto_ack_stream = futures::StreamExt::then(subscriber, move |message| {
            let queue = queue.clone();
            let topic = topic.clone();
            let process_message = process_message.clone();
            
            async move {
                // Process the message first
                if let Err(e) = process_message(message.clone()).await {
                    return Err(e);
                }

                // If processing succeeded, remove the message
                let message_key = format!("message:{}:{}", topic, message.id);
                let topic_key = format!("topic:{}:messages", topic);

                // Execute Redis commands
                let mut conn = queue.redis.clone();
                
                // Use a multi/exec transaction to ensure atomicity
                redis::cmd("MULTI").query_async::<_, ()>(&mut conn).await.map_err(MessageError::RedisError)?;
                
                // Delete the message data
                redis::cmd("DEL")
                    .arg(&message_key)
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(MessageError::RedisError)?;
                
                // Remove the message ID from the topic's list
                redis::cmd("LREM")
                    .arg(&topic_key)
                    .arg(0) // remove all occurrences
                    .arg(&message.id.to_string())
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(MessageError::RedisError)?;
                
                // Execute the transaction
                redis::cmd("EXEC").query_async::<_, ()>(&mut conn).await.map_err(MessageError::RedisError)?;

                Ok(())
            }
        });

        Ok(Box::pin(auto_ack_stream))
    }

    pub async fn auto_ack_subscribe_with_filter<F, P, Fut>(
        &self,
        topic: String,
        filter: F,
        process_message: P,
    ) -> Result<AutoAckStream, MessageError>
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
        P: Fn(Message) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<(), MessageError>> + Send + 'static,
    {
        let subscriber = self.subscribe_with_filter(&topic, filter).await?;
        let queue = self.clone();

        let auto_ack_stream = futures::StreamExt::then(subscriber, move |message| {
            let queue = queue.clone();
            let topic = topic.clone();
            let process_message = process_message.clone();
            
            async move {
                // Process the message first
                if let Err(e) = process_message(message.clone()).await {
                    return Err(e);
                }

                // If processing succeeded, remove the message
                let message_key = format!("message:{}:{}", topic, message.id);
                let topic_key = format!("topic:{}:messages", topic);

                // Execute Redis commands
                let mut conn = queue.redis.clone();
                
                // Use a multi/exec transaction to ensure atomicity
                redis::cmd("MULTI").query_async::<_, ()>(&mut conn).await.map_err(MessageError::RedisError)?;
                
                // Delete the message data
                redis::cmd("DEL")
                    .arg(&message_key)
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(MessageError::RedisError)?;
                
                // Remove the message ID from the topic's list
                redis::cmd("LREM")
                    .arg(&topic_key)
                    .arg(0) // remove all occurrences
                    .arg(&message.id.to_string())
                    .query_async::<_, ()>(&mut conn)
                    .await
                    .map_err(MessageError::RedisError)?;
                
                // Execute the transaction
                redis::cmd("EXEC").query_async::<_, ()>(&mut conn).await.map_err(MessageError::RedisError)?;

                Ok(())
            }
        });

        Ok(Box::pin(auto_ack_stream))
    }

    async fn save_to_redis(&self, topic: &str, message: &Message) -> Result<(), MessageError> {
        let mut conn = self.redis.clone();
        
        // Store message in Redis
        let message_key = format!("message:{}:{}", topic, message.id);
        let message_json = serde_json::to_string(message)?;
        
        // Store message data
        conn.set::<_, _, ()>(&message_key, &message_json).await.map_err(MessageError::RedisError)?;
        
        // Add message ID to topic's message list
        let topic_key = format!("topic:{}:messages", topic);
        conn.lpush::<_, _, ()>(&topic_key, message.id.to_string()).await.map_err(MessageError::RedisError)?;

        Ok(())
    }

    pub async fn get_messages(&self, topic: &str, count: isize) -> Result<Vec<Message>, MessageError> {
        let mut conn = self.redis.clone();
        let topic_key = format!("topic:{}:messages", topic);
        
        // Get message IDs from Redis
        let message_ids: Vec<String> = conn.lrange(&topic_key, 0, count - 1).await.map_err(MessageError::RedisError)?;
        
        let mut messages = Vec::new();
        for id in message_ids {
            let message_key = format!("message:{}:{}", topic, id);
            if let Ok(message_json) = conn.get::<_, String>(&message_key).await.map_err(MessageError::RedisError) {
                if let Ok(message) = serde_json::from_str::<Message>(&message_json) {
                    messages.push(message);
                }
            }
        }
        
        Ok(messages)
    }

    pub async fn remove_message(&self, topic: &str, message_id: &str) -> Result<(), MessageError> {
        let mut conn = self.redis.clone();
        
        // Remove message data
        let message_key = format!("message:{}:{}", topic, message_id);
        conn.del::<_, ()>(&message_key).await.map_err(MessageError::RedisError)?;
        
        // Remove message ID from topic's message list
        let topic_key = format!("topic:{}:messages", topic);
        conn.lrem::<_, _, ()>(&topic_key, 1, message_id).await.map_err(MessageError::RedisError)?;
        
        Ok(())
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
                        if let Err(e) = conn.del::<_, ()>(&topic_key).await {
                            log::error!("Failed to clean up topic {}: {}", topic, e);
                        }
                    }
                }
            }
        });
    }
} 