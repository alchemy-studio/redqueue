use crate::{Message, RedQueue};
use std::time::Duration;
use tokio::time::sleep;
use futures::StreamExt;

#[tokio::test]
async fn test_message_polling() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting test_message_polling");
    
    // Initialize RedQueue
    let redis_url = "redis://127.0.0.1/";
    let cleanup_interval = Duration::from_secs(60);
    let queue = RedQueue::new(redis_url, cleanup_interval).await?;
    println!("RedQueue initialized");

    // Test basic message polling
    let topic = "test_polling";
    let mut subscriber = queue.subscribe(topic).await?;
    println!("Subscribed to topic: {}", topic);

    // Start a background task to publish messages
    let queue_clone = queue.clone();
    let topic_clone = topic.to_string();
    tokio::spawn(async move {
        println!("Starting to publish messages");
        for i in 0..5 {
            let payload = format!("Message {}", i).into_bytes();
            let message = Message::new(topic_clone.clone(), payload);
            match queue_clone.publish(&topic_clone, message).await {
                Ok(_) => println!("Published message {}", i),
                Err(e) => println!("Error publishing message {}: {:?}", i, e),
            }
            sleep(Duration::from_millis(100)).await;
        }
        println!("Finished publishing messages");
    });

    // Poll for messages with timeout
    let mut received_messages = Vec::new();
    let timeout = Duration::from_secs(5);

    println!("Starting to receive messages");
    while let Some(message) = tokio::time::timeout(timeout, subscriber.next()).await? {
        println!("Received a message");
        received_messages.push(message);
        if received_messages.len() >= 5 {
            break;
        }
    }
    println!("Finished receiving messages. Count: {}", received_messages.len());

    // Verify received messages
    assert_eq!(received_messages.len(), 5, "Did not receive all 5 messages");
    for (i, msg) in received_messages.iter().enumerate() {
        let expected_payload = format!("Message {}", i);
        assert_eq!(String::from_utf8_lossy(&msg.payload), expected_payload);
    }
    println!("Message verification passed");

    // Test message persistence
    let stored_messages = queue.get_messages(topic, 5).await?;
    assert_eq!(stored_messages.len(), 5, "Stored message count mismatch");
    println!("Message persistence test passed");

    // Test filtered messages with a new topic
    let filtered_topic = "test_filtered";
    println!("\nStarting filtered message test with topic: {}", filtered_topic);

    // Create filtered subscriber first
    let mut filtered_subscriber = queue
        .subscribe_with_filter(filtered_topic, |msg| {
            let payload = String::from_utf8_lossy(&msg.payload);
            payload.ends_with("0") || payload.ends_with("2") || payload.ends_with("4")
        })
        .await?;
    println!("Created filtered subscriber");

    // Publish messages for filtering
    println!("Publishing messages for filter test");
    for i in 0..6 {
        let payload = format!("FilteredMessage {}", i).into_bytes();
        let message = Message::new(filtered_topic.to_string(), payload);
        queue.publish(filtered_topic, message).await?;
        println!("Published message: FilteredMessage {}", i);
        sleep(Duration::from_millis(100)).await;
    }
    println!("Finished publishing filtered messages");

    // Collect filtered messages
    let mut filtered_messages = Vec::new();
    println!("Starting to receive filtered messages");
    
    let filter_timeout = Duration::from_secs(2);
    while let Ok(Some(message)) = tokio::time::timeout(filter_timeout, filtered_subscriber.next()).await {
        let payload = String::from_utf8_lossy(&message.payload);
        println!("Received filtered message: {}", payload);
        filtered_messages.push(message);
        if filtered_messages.len() >= 3 {
            break;
        }
    }
    println!("Finished receiving filtered messages. Count: {}", filtered_messages.len());

    // Verify filtered messages (should receive messages 0, 2, and 4)
    assert_eq!(filtered_messages.len(), 3, "Did not receive all filtered messages");
    for msg in filtered_messages {
        let payload = String::from_utf8_lossy(&msg.payload);
        assert!(payload.ends_with("0") || payload.ends_with("2") || payload.ends_with("4"),
                "Filtered message did not match expected pattern");
        println!("Verified filtered message: {}", payload);
    }
    println!("Filter test passed");

    println!("Test completed successfully");
    Ok(())
} 