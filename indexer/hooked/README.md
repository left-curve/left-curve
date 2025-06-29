# HookedIndexer

A composable indexer that allows you to combine multiple indexers and share data between them through a context system.

## Design

The HookedIndexer uses the existing `Indexer` trait from `grug_app::Indexer` directly. Since we modified the trait to use `&dyn Storage` instead of generic parameters, it's now fully dyn-compatible and can be used with trait objects without any wrapper traits.

### Basic Usage

```rust
use indexer_hooked::HookedIndexer;
use grug_app::Indexer;

// Define your custom indexer using the standard Indexer trait
struct SqlIndexer {
    // Your SQL indexer implementation
}

impl Indexer for SqlIndexer {
    type Error = std::convert::Infallible;

    fn start(&mut self, storage: &dyn Storage) -> Result<(), Self::Error> {
        // Initialize your SQL indexer
        Ok(())
    }

    fn index_block(
        &self,
        block: &Block,
        block_outcome: &BlockOutcome,
    ) -> Result<(), Self::Error> {
        // Index to SQL database
        Ok(())
    }

    // ... implement other required methods
}

// Usage - HookedIndexer automatically wraps your Indexer with error conversion
let mut hooked_indexer = HookedIndexer::new();
hooked_indexer
    .add_indexer(SqlIndexer::new())
    .add_indexer(ClickHouseIndexer::new())
    .add_indexer(LoggingIndexer::new());

// Use as a standard Indexer
let storage = get_storage();
hooked_indexer.start(&storage)?;
```

### Error Conversion

The `IndexerAdapter` handles the conversion from any `Indexer` implementation to `Indexer<Error = HookedIndexerError>`, providing error conversion and thread-safe access through a `Mutex`.

## Context System

The `IndexerContext` provides a type-safe way to share data between indexers using [anymap](https://crates.io/crates/anymap), a popular Rust crate for type-safe storage:

```rust
// Access the shared context
let mut context = hooked_indexer.context_mut();

// Store typed data
context.set_data(DatabaseConnection::new(...));
context.set_property("last_block_hash".to_string(), block_hash);

// Retrieve data in other indexers (via middleware or custom logic)
if let Some(db) = context.get_data::<DatabaseConnection>() {
    // Use the shared database connection (returns a clone for Clone types)
}

let block_hash = context.get_property("last_block_hash");
```

### TypeMap Implementation

Under the hood, HookedIndexer uses the `anymap` crate which provides:

- Type-safe storage and retrieval
- Zero-cost type erasure
- Thread-safe operations with Arc<RwLock<>>
- Well-tested and maintained implementation

Note: The `get_data<T>()` method returns `Option<T>` (cloned values) rather than `Option<&T>` due to the thread-safe design using `Arc<RwLock<TypeMap>>`.

## Features

- **Standard Indexer Compatibility**: Uses the existing `grug_app::Indexer` trait
- **Type-safe data sharing**: Use `TypeMap` to share typed data between indexers
- **Metadata support**: Store arbitrary key-value metadata in the context
- **Error handling**: Unified error handling across all indexers
- **Lifecycle management**: Proper startup and shutdown coordination
- **Middleware support**: Built-in adapter pattern for wrapping existing indexers

## Architecture Benefits

1. **Standard Compatibility**: Uses the existing `Indexer` trait - no new interfaces to learn
2. **Separation of Concerns**: Each indexer handles one specific concern (SQL, ClickHouse, logging, etc.)
3. **Composability**: Mix and match different indexers as needed
4. **Testability**: Test each indexer independently using standard patterns
5. **Data Sharing**: Efficient data sharing between indexers through shared context
6. **Minimal Overhead**: The adapter pattern adds minimal overhead

## Example: Multiple Indexers

```rust
// Create individual indexers
let logging_indexer = LoggingIndexer::new();
let sql_indexer = SqlIndexer::new();
let metrics_indexer = MetricsIndexer::new();

// Keep references for checking results
let logs = logging_indexer.logs.clone();
let metrics = metrics_indexer.total_blocks.clone();

// Compose them
let mut hooked_indexer = HookedIndexer::new();
hooked_indexer
    .add_indexer(logging_indexer)
    .add_indexer(sql_indexer)
    .add_indexer(metrics_indexer);

// Add some shared context data
hooked_indexer.context_mut().set_data(SharedConfig::new());

// Use as standard Indexer
hooked_indexer.start(&storage)?;

// Process blocks
hooked_indexer.pre_indexing(1)?;
hooked_indexer.index_block(&block, &block_outcome)?;
hooked_indexer.post_indexing(1, Box::new(mock_querier))?;

hooked_indexer.shutdown()?;

// Check results
println!("Logs: {:?}", logs.read().unwrap());
println!("Blocks processed: {}", metrics.read().unwrap());
```

## Limitations and Design Notes

### Current Design Trade-offs

1. **Type Erasure**: Uses `&dyn Storage` instead of generic storage for dyn compatibility
2. **Error Conversion**: Converts `Indexer::Error` to `String` for uniform error handling
3. **Querier Boxing**: Requires `Box<dyn QuerierProvider>` which may have lifetime implications

### Direct Indexer Usage

The `Indexer` trait is now fully dyn-compatible because:

- We changed the `start` method to use `&dyn Storage` instead of generic types
- All trait methods are now object-safe for `dyn` usage
- No wrapper traits are needed

### Performance Considerations

- The wrapper adds minimal overhead (single function call indirection)
- Context operations use `Arc<RwLock<>>` for thread safety
- Type erasure may prevent some compiler optimizations

## Examples

See the `/examples` directory for complete working examples:

- `basic_usage.rs`: Shows how to compose multiple indexers with shared context

## Testing

Run the test suite:

```bash
cargo test
```

Run the examples:

```bash
cargo run --example basic_usage
```
