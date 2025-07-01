# HookedIndexer

A composable indexer that allows you to combine multiple indexers like middleware in web frameworks. Each indexer in the composition handles a specific concern (SQL storage, ClickHouse analytics, Redis caching, logging, etc.) while sharing data through a typed context system.

## Design Philosophy

The HookedIndexer implements the standard `Indexer` trait and acts as a coordinator for multiple child indexers. It calls each indexer in sequence for every indexing operation, providing a shared context for data exchange.

## Basic Usage

```rust
use indexer_hooked::HookedIndexer;
use grug_app::Indexer;

// Create your indexers
let sql_indexer = SqlIndexer::new();
let clickhouse_indexer = ClickHouseIndexer::new();
let logging_indexer = LoggingIndexer::new();

// Compose them
let mut hooked_indexer = HookedIndexer::new();
hooked_indexer
    .add_indexer(sql_indexer)
    .add_indexer(clickhouse_indexer)
    .add_indexer(logging_indexer);

// Use as a standard Indexer
hooked_indexer.start(&storage)?;
hooked_indexer.index_block(&block, &block_outcome)?;
```

## Context System

The `IndexerContext` provides type-safe data sharing between indexers using `http::Extensions`:

```rust
// Store typed data in the context
hooked_indexer.context_mut().data().lock().unwrap().insert(DatabasePool::new());
hooked_indexer.context_mut().set_property("version".to_string(), "1.0".to_string());

// Access from any indexer that has access to the context
let pool = context.data().lock().unwrap().get::<DatabasePool>().cloned();
let version = context.get_property("version");
```

## Adding Context Data

Use `add_indexer_with_context` to store additional data when adding an indexer:

```rust
let sql_indexer = SqlIndexer::new(db_pool.clone());

hooked_indexer.add_indexer_with_context(sql_indexer, |indexer, context| {
    // Store the database pool for other indexers to use
    context.data().lock().unwrap().insert(db_pool.clone());
});
```

## Features

- **Standard Compatibility**: Uses the existing `grug_app::Indexer` trait
- **Type-safe Data Sharing**: Uses `http::Extensions` for sharing typed data between indexers
- **String Metadata**: Simple key-value metadata storage
- **Error Conversion**: Automatic error conversion via `Into<HookedIndexerError>`
- **Lifecycle Management**: Proper startup and shutdown coordination
- **Sequential Execution**: Indexers are called in the order they were added

## Architecture Benefits

1. **Separation of Concerns**: Each indexer handles one responsibility
2. **Composability**: Mix and match indexers as needed
3. **Testability**: Test each indexer independently
4. **Data Sharing**: Efficient typed data sharing via `Extensions`
5. **Standard Interface**: Uses the existing `Indexer` trait

## Example: Multiple Storage Systems

```rust
// Individual indexers for different storage systems
let postgres_indexer = PostgresIndexer::new(pg_pool);
let clickhouse_indexer = ClickHouseIndexer::new(ch_client);
let redis_indexer = RedisIndexer::new(redis_client);
let metrics_indexer = MetricsIndexer::new();

// Compose them
let mut hooked_indexer = HookedIndexer::new();
hooked_indexer
    .add_indexer(postgres_indexer)     // Primary storage
    .add_indexer(clickhouse_indexer)   // Analytics
    .add_indexer(redis_indexer)        // Caching
    .add_indexer(metrics_indexer);     // Monitoring

// Add shared configuration
hooked_indexer.context_mut().data().lock().unwrap().insert(SharedConfig::new());

// Use normally
hooked_indexer.start(&storage)?;
// All indexers will be called for each block
hooked_indexer.index_block(&block, &block_outcome)?;
```

## Error Handling

Indexer errors are automatically converted to `HookedIndexerError` via the `Into` trait. If any indexer fails, the entire operation fails with details about which indexer caused the error.

## Examples

See the `/examples` directory for complete working examples.

## Testing

```bash
cargo test
```
