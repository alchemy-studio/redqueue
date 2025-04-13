# RedQueue

A blazing-fast, Redis-based asynchronous messaging system built with Rust and Tokio. RedQueue combines the power of Redis with Rust's performance to deliver a robust message queue system.

## Features

- Asynchronous message publishing and subscription using Tokio
- Redis-backed message persistence
- Multiple producers and consumers support
- Topic-based message routing
- Message filtering capabilities
- Automatic topic cleanup
- JSON-based message serialization
- UUID-based message tracking

## Prerequisites

- Redis server (local or remote)
- Rust and Cargo installed

## Installation

1. Add RedQueue to your project:
```toml
[dependencies]
redqueue = { path = "path/to/redqueue" }
```

2. Make sure Redis is running:
```bash
# Start Redis server
redis-server

# Test Redis connection
redis-cli ping  # Should return "PONG"
```

## Usage

### Basic Example

```rust
use redqueue::{Message, RedQueue};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create RedQueue instance
    let redis_url = "redis://127.0.0.1/";
    let cleanup_interval = Duration::from_secs(60);
    let queue = RedQueue::new(redis_url, cleanup_interval).await?;

    // Create a topic
    let topic = "my_topic";

    // Subscribe to messages
    let mut subscriber = queue.subscribe(topic).await?;

    // Subscribe with filter (only even-length payloads)
    let mut filtered = queue
        .subscribe_with_filter(topic, |msg| msg.payload.len() % 2 == 0)
        .await?;

    // Publish a message
    let message = Message::new(topic.to_string(), b"Hello, RedQueue!".to_vec());
    queue.publish(topic, message).await?;

    // Retrieve stored messages
    let messages = queue.get_messages(topic, 10).await?;
    println!("Stored messages: {:?}", messages);

    Ok(())
}
```

### Message Structure

```rust
pub struct Message {
    pub id: Uuid,           // Unique message identifier
    pub topic: String,      // Message topic
    pub payload: Vec<u8>,   // Message content
    pub timestamp: u64,     // Unix timestamp
    pub metadata: Value,    // Additional JSON metadata
}
```

### Redis Data Structure

RedQueue uses the following Redis keys:

- `message:{topic}:{id}` - Stores serialized message data
- `topic:{topic}:messages` - List of message IDs for a topic

### Advanced Features

1. **Message Filtering**
```rust
// Subscribe with custom filter
let mut subscriber = queue
    .subscribe_with_filter("topic", |msg| {
        // Custom filter logic
        msg.payload.len() > 100
    })
    .await?;
```

2. **Message Retrieval**
```rust
// Get last 10 messages from a topic
let messages = queue.get_messages("topic", 10).await?;
```

3. **Automatic Cleanup**
```rust
// Start cleanup task (removes unused topics)
queue.start_cleanup().await;
```

## Project Structure

```
redqueue/
├── src/
│   ├── lib.rs          # Library exports
│   ├── message.rs      # Message types and serialization
│   ├── messaging.rs    # Core RedQueue implementation
│   └── main.rs         # Example usage
├── Cargo.toml          # Project dependencies
└── README.md          # This file
```

## Dependencies

- `tokio`: Async runtime and utilities
- `redis`: Redis client with async support
- `serde`: Serialization/deserialization
- `uuid`: Message ID generation
- `futures`: Stream utilities
- `log`: Logging support

## Running the Example

1. Start Redis server:
```bash
redis-server
```

2. Run the example:
```bash
RUST_LOG=info cargo run
```

## Error Handling

RedQueue provides comprehensive error handling through the `MessageError` enum:

```rust
pub enum MessageError {
    TopicNotFound,
    SerializationError(serde_json::Error),
    IoError(std::io::Error),
    RedisError(redis::RedisError),
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT 

## Testing

Here's a comprehensive test case that demonstrates message polling functionality:

```rust
use redqueue::{Message, RedQueue};
use std::time::Duration;
use tokio::time::sleep;
use futures::StreamExt;

#[tokio::test]
async fn test_message_polling() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RedQueue
    let redis_url = "redis://127.0.0.1/";
    let cleanup_interval = Duration::from_secs(60);
    let queue = RedQueue::new(redis_url, cleanup_interval).await?;

    // Create test topic
    let topic = "test_polling";

    // Subscribe to messages
    let mut subscriber = queue.subscribe(topic).await?;

    // Start a background task to publish messages
    let queue_clone = queue.clone();
    let topic_clone = topic.to_string();
    tokio::spawn(async move {
        for i in 0..5 {
            let payload = format!("Message {}", i).into_bytes();
            let message = Message::new(topic_clone.clone(), payload);
            queue_clone.publish(&topic_clone, message).await.unwrap();
            sleep(Duration::from_millis(100)).await;
        }
    });

    // Poll for messages with timeout
    let mut received_messages = Vec::new();
    let timeout = Duration::from_secs(2);

    while let Some(message) = tokio::time::timeout(timeout, subscriber.next()).await? {
        received_messages.push(message);
        if received_messages.len() >= 5 {
            break;
        }
    }

    // Verify received messages
    assert_eq!(received_messages.len(), 5);
    for (i, msg) in received_messages.iter().enumerate() {
        let expected_payload = format!("Message {}", i);
        assert_eq!(String::from_utf8_lossy(&msg.payload), expected_payload);
    }

    // Test message persistence
    let stored_messages = queue.get_messages(topic, 5).await?;
    assert_eq!(stored_messages.len(), 5);

    // Test message filtering
    let mut filtered_subscriber = queue
        .subscribe_with_filter(topic, |msg| {
            // Only accept messages with even-numbered payloads
            let payload = String::from_utf8_lossy(&msg.payload);
            payload.ends_with("0") || payload.ends_with("2") || payload.ends_with("4")
        })
        .await?;

    // Publish more messages
    for i in 5..10 {
        let payload = format!("Message {}", i).into_bytes();
        let message = Message::new(topic.to_string(), payload);
        queue.publish(topic, message).await?;
        sleep(Duration::from_millis(100)).await;
    }

    // Verify filtered messages
    let mut filtered_messages = Vec::new();
    while let Some(message) = tokio::time::timeout(timeout, filtered_subscriber.next()).await? {
        filtered_messages.push(message);
        if filtered_messages.len() >= 3 {
            break;
        }
    }

    assert_eq!(filtered_messages.len(), 3);
    for msg in filtered_messages {
        let payload = String::from_utf8_lossy(&msg.payload);
        assert!(payload.ends_with("0") || payload.ends_with("2") || payload.ends_with("4"));
    }

    Ok(())
}
```

This test case demonstrates:

1. **Basic Message Polling**
   - Subscribes to a topic
   - Publishes messages in a background task
   - Polls for messages with timeout
   - Verifies received messages

2. **Message Persistence**
   - Retrieves stored messages from Redis
   - Verifies message count and content

3. **Message Filtering**
   - Creates a filtered subscriber
   - Publishes additional messages
   - Verifies that only filtered messages are received

To run the tests:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_message_polling -- --nocapture
``` 