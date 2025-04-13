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
use futures::StreamExt;

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
            if let Err(e) = queue_clone.publish(topic, message).await {
                eprintln!("Failed to publish message: {}", e);
            }
            sleep(Duration::from_secs(1)).await;
        }
    });

    // Process messages from subscribers
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            println!("Regular subscriber received: {:?}", message);
        }
    });

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
│   ├── lib.rs          # Library exports and public API
│   ├── message.rs      # Message types and serialization
│   ├── messaging.rs    # Core RedQueue implementation
│   └── main.rs         # Example usage
├── tests/
│   └── messaging_integration.rs  # Integration tests
├── Cargo.toml          # Project dependencies
└── README.md          # This file
```

## Dependencies

- `tokio` (1.36): Async runtime and utilities
- `redis` (0.24): Redis client with async support
- `serde` (1.0): Serialization/deserialization
- `serde_json` (1.0): JSON support
- `futures` (0.3): Stream utilities
- `async-trait` (0.1): Async trait support
- `thiserror` (1.0): Error handling
- `uuid` (1.7): Message ID generation
- `log` (0.4): Logging support
- `env_logger` (0.10): Logger implementation

## Running Tests

1. Start Redis server:
```bash
redis-server
```

2. Run the tests:
```bash
cargo test
```

The test suite includes comprehensive integration tests that verify:
- Basic message publishing and subscription
- Message filtering functionality
- Message persistence and retrieval
- Topic-based message routing
- Cleanup functionality

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