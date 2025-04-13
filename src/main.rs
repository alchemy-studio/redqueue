use redqueue::{message::Message, messaging::RedQueue};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Create RedQueue instance
    let redis_url = "redis://127.0.0.1/";
    let cleanup_interval = Duration::from_secs(60);
    let queue = RedQueue::new(redis_url, cleanup_interval).await?;

    // Start cleanup task
    queue.start_cleanup().await;

    // Create a topic
    let topic = "test_topic";

    // Create a subscriber
    let mut subscriber = queue.subscribe(topic).await?;

    // Create a subscriber with filter
    let mut filtered_subscriber = queue
        .subscribe_with_filter(topic, |msg| {
            // Only accept messages with even payload length
            msg.payload.len() % 2 == 0
        })
        .await?;

    // Spawn publisher task
    let queue_clone = queue.clone();
    tokio::spawn(async move {
        for i in 0..5 {
            let payload = format!("Message {}", i).into_bytes();
            let message = Message::new(topic.to_string(), payload);
            queue_clone.publish(topic, message).await.unwrap();
            sleep(Duration::from_secs(1)).await;
        }
    });

    // Spawn regular subscriber task
    let queue_clone = queue.clone();
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            println!("Regular subscriber received: {:?}", message);
        }
    });

    // Spawn filtered subscriber task
    tokio::spawn(async move {
        while let Some(message) = filtered_subscriber.next().await {
            println!("Filtered subscriber received: {:?}", message);
        }
    });

    // Wait a bit and then retrieve stored messages
    sleep(Duration::from_secs(2)).await;
    let stored_messages = queue.get_messages(topic, 5).await?;
    println!("Stored messages: {:?}", stored_messages);

    // Keep the main thread alive
    sleep(Duration::from_secs(10)).await;

    Ok(())
}
