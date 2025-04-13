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