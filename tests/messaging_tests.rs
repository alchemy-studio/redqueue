use redqueue::message::{Message, MessageError};
use redqueue::messaging::RedQueue;
use redis::{aio::ConnectionManager, Client};
use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};
use tokio::time::sleep;
use futures::StreamExt;

async fn cleanup_redis(redis_url: &str) -> Result<(), MessageError> {
    let client = Client::open(redis_url).map_err(MessageError::RedisError)?;
    let mut conn = ConnectionManager::new(client).await.map_err(MessageError::RedisError)?;
    redis::cmd("FLUSHDB").query_async::<_, ()>(&mut conn).await.map_err(MessageError::RedisError)?;
    Ok(())
}

#[tokio::test]
async fn test_auto_ack_subscribe_success() {
    let redis_url = "redis://127.0.0.1/";
    cleanup_redis(redis_url).await.unwrap();
    
    let queue = RedQueue::new(redis_url, Duration::from_secs(1))
        .await
        .unwrap();
    let topic = "test_topic".to_string();
    
    // Create a message processor that always succeeds
    let process_message = |msg: Message| async move {
        println!("Processing message: {:?}", msg);
        Ok(())
    };

    // Subscribe with auto-ack
    let mut stream = queue.auto_ack_subscribe(topic.clone(), process_message)
        .await
        .unwrap();

    // Publish a message
    let message = Message::new(topic.clone(), "test content".as_bytes().to_vec());
    queue.publish(&topic, message.clone()).await.unwrap();
    println!("Published message with ID: {}", message.id);

    // Wait for message to be processed and stream to return
    let result = stream.next().await.unwrap();
    assert!(result.is_ok(), "Message processing failed");
    println!("Message processing completed");

    // Wait a bit longer for Redis cleanup
    sleep(Duration::from_millis(500)).await;

    // Verify message was processed and removed
    let messages = queue.get_messages(&topic, 10).await.unwrap();
    if !messages.is_empty() {
        println!("Found {} messages in queue:", messages.len());
        for msg in &messages {
            println!("  Message ID: {}", msg.id);
        }
    }
    assert!(messages.is_empty(), "Messages were not removed from Redis");
}

#[tokio::test]
async fn test_auto_ack_subscribe_error() {
    let redis_url = "redis://127.0.0.1/";
    cleanup_redis(redis_url).await.unwrap();
    
    let queue = RedQueue::new(redis_url, Duration::from_secs(1))
        .await
        .unwrap();
    let topic = "test_topic_error".to_string();
    
    // Create a message processor that always fails
    let process_message = |msg: Message| async move {
        println!("Processing message: {:?}", msg);
        Err(MessageError::RedisError(redis::RedisError::from((
            redis::ErrorKind::IoError,
            "Test error",
        ))))
    };

    // Subscribe with auto-ack
    let mut stream = queue.auto_ack_subscribe(topic.clone(), process_message)
        .await
        .unwrap();

    // Publish a message
    let message = Message::new(topic.clone(), "test content".as_bytes().to_vec());
    queue.publish(&topic, message.clone()).await.unwrap();
    println!("Published message with ID: {}", message.id);

    // Wait for message to be processed and stream to return
    let result = stream.next().await.unwrap();
    assert!(result.is_err(), "Expected message processing to fail");
    println!("Message processing failed as expected");

    // Wait a bit longer for any Redis operations
    sleep(Duration::from_millis(500)).await;

    // Verify message is still in the queue due to processing failure
    let messages = queue.get_messages(&topic, 10).await.unwrap();
    assert_eq!(messages.len(), 1, "Expected one message to remain in queue");
    assert_eq!(messages[0].id, message.id, "Message ID mismatch");
}

#[tokio::test]
async fn test_auto_ack_subscribe_multiple_messages() {
    let redis_url = "redis://127.0.0.1/";
    cleanup_redis(redis_url).await.unwrap();
    
    let queue = RedQueue::new(redis_url, Duration::from_secs(1))
        .await
        .unwrap();
    let topic = "test_topic_multiple".to_string();
    
    let processed_count = std::sync::Arc::new(AtomicUsize::new(0));
    let process_message = {
        let processed_count = processed_count.clone();
        move |msg: Message| {
            let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
            println!("Processing message {}: {:?}", count, msg);
            async move { Ok(()) }
        }
    };

    // Subscribe with auto-ack
    let mut stream = queue.auto_ack_subscribe(topic.clone(), process_message)
        .await
        .unwrap();

    // Publish multiple messages
    let mut message_ids = Vec::new();
    for i in 0..3 {
        let message = Message::new(topic.clone(), format!("test content {}", i).as_bytes().to_vec());
        message_ids.push(message.id.clone());
        queue.publish(&topic, message).await.unwrap();
        println!("Published message {} with ID: {}", i + 1, message_ids[i]);
    }

    // Wait for all messages to be processed
    for (i, _) in message_ids.iter().enumerate() {
        let result = stream.next().await.unwrap();
        assert!(result.is_ok(), "Message {} processing failed", i + 1);
        println!("Message {} processing completed", i + 1);
    }

    // Wait a bit longer for Redis cleanup
    sleep(Duration::from_millis(500)).await;

    // Verify all messages were processed and removed
    let messages = queue.get_messages(&topic, 10).await.unwrap();
    if !messages.is_empty() {
        println!("Found {} messages in queue:", messages.len());
        for msg in &messages {
            println!("  Message ID: {}", msg.id);
        }
    }
    assert!(messages.is_empty(), "Messages were not removed from Redis");

    // Verify the correct number of messages were processed
    assert_eq!(processed_count.load(Ordering::SeqCst), 3, "Not all messages were processed");
} 