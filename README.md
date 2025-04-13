# Rust Tokio Messaging System with Redis

A simple, asynchronous messaging system built with Rust, Tokio, and Redis, supporting publish/subscribe patterns with persistence.

## Features

- Asynchronous message publishing and subscription
- Multiple producers and consumers support
- Message persistence using Redis
- Topic-based message routing
- Message filtering capabilities
- Automatic topic cleanup
- Message acknowledgment support
- Redis-based message storage and retrieval

## Prerequisites

- Redis server running locally or accessible via network
- Rust and Cargo installed

## Project Structure

```
messaging_system/
├── src/
│   ├── main.rs         # Example usage and entry point
│   ├── message.rs      # Message types and structures
│   └── messaging.rs    # Core messaging system implementation
├── Cargo.toml          # Project dependencies
└── README.md           # This file
```

## Usage

1. Create a new messaging system instance:
```rust
let redis_url = "redis://127.0.0.1/";
let cleanup_interval = Duration::from_secs(60);
let messaging = MessagingSystem::new(redis_url, cleanup_interval).await?;
```

2. Publish messages:
```rust
let message = Message::new("topic_name".to_string(), payload);
messaging.publish("topic_name", message).await?;
```

3. Subscribe to messages:
```rust
let mut subscriber = messaging.subscribe("topic_name").await?;
while let Some(message) = subscriber.next().await {
    // Handle message
}
```

4. Subscribe with filter:
```rust
let mut filtered_subscriber = messaging
    .subscribe_with_filter("topic_name", |msg| {
        // Filter condition
        true
    })
    .await?;
```

5. Retrieve stored messages:
```rust
let messages = messaging.get_messages("topic_name", 10).await?;
```

## Redis Data Structure

The system uses the following Redis data structures:

- `message:{topic}:{id}`: Hash containing message data
- `topic:{topic}:messages`: List containing message IDs for a topic

## Dependencies

- tokio: Async runtime
- redis: Redis client
- serde: Serialization/deserialization
- uuid: Message ID generation
- futures: Stream utilities
- log: Logging support

## Building and Running

1. Start Redis server:
```bash
redis-server
```

2. Build and run the project:
```bash
cargo run
```

## License

MIT 