# HookedIndexer

A composable indexer that allows you to combine multiple indexers like middleware in web frameworks. Each indexer in the composition handles a specific concern (SQL storage, ClickHouse analytics, Redis caching, logging, etc.) while maintaining complete independence.

## Design Philosophy

The HookedIndexer implements the standard `Indexer` trait and acts as a coordinator for multiple child indexers. It calls each indexer in sequence for every indexing operation, ensuring all indexers process every block.

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

## Features

- **Standard Compatibility**: Uses the existing `grug_app::Indexer` trait
- **Simple Composition**: Just add indexers and they'll all be called
- **Independent Operation**: Each indexer operates independently
- **Error Propagation**: If any indexer fails, the entire operation fails
- **Lifecycle Management**: Proper startup and shutdown coordination
- **Sequential Execution**: Indexers are called in the order they were added

## Architecture Benefits

1. **Separation of Concerns**: Each indexer handles one responsibility
2. **Composability**: Mix and match indexers as needed
3. **Testability**: Test each indexer independently
4. **Standard Interface**: Uses the existing `Indexer` trait
5. **Zero Overhead**: Simple delegation with no additional complexity

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

// Use normally - all indexers will be called for each block
hooked_indexer.start(&storage)?;
hooked_indexer.index_block(&block, &block_outcome)?;
```

## Error Handling

Indexer errors use the standard `IndexerError` type. If any indexer fails, the entire operation fails immediately, maintaining consistency across all indexers.

## Examples

See the `/examples` directory for complete working examples.

## Testing

```bash
cargo test
```
