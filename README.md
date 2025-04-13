# Rust Tokio Messaging System

A simple, asynchronous messaging system built with Rust and Tokio, supporting publish/subscribe patterns with persistence.

## Features

- Asynchronous message publishing and subscription
- Multiple producers and consumers support
- Message persistence to disk
- Topic-based message routing
- Message filtering capabilities
- Automatic topic cleanup
- Message acknowledgment support

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
let storage_path = std::env::current_dir()?.join("messages");
let cleanup_interval = Duration::from_secs(60);
let messaging = MessagingSystem::new(storage_path, cleanup_interval).await?;
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

## Dependencies

- tokio: Async runtime
- serde: Serialization/deserialization
- uuid: Message ID generation
- futures: Stream utilities
- log: Logging support

## Building and Running

```bash
# Build the project
cargo build

# Run the example
cargo run
```

## License

MIT 