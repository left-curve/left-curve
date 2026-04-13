# Indexer and Node Architecture

This chapter covers the indexer pipeline, SQL schema, GraphQL API, and the Dango CLI
that wires everything together.

## 1. Indexer Design

The indexer is a **read-only, non-consensus** component that observes state transitions
and writes structured data to external databases. It **cannot affect consensus** -- its
operations run after transaction execution and are never on the critical path for state
commitment.

### Indexer trait

```rust
// grug/app/src/traits/indexer.rs
#[async_trait]
pub trait Indexer {
    async fn start(&mut self, storage: &dyn Storage) -> IndexerResult<()>;
    async fn pre_indexing(&self, block_height: u64, ctx: &mut IndexerContext) -> IndexerResult<()>;
    async fn index_block(&self, block: ..., outcome: ..., ctx: &mut IndexerContext) -> IndexerResult<()>;
    async fn post_indexing(&self, block_height: u64, ..., ctx: &mut IndexerContext) -> IndexerResult<()>;
    async fn shutdown(&self) -> IndexerResult<()>;
}
```

### Call sequence in FinalizeBlock

```text
1. pre_indexing()     ← BEFORE transaction execution
2. Execute all txs
3. Execute cronjobs
4. Remove orphaned codes
5. db.flush_but_not_commit()  ← State root computed
6. index_block()      ← AFTER execution, BEFORE commit
7. [Commit happens separately in do_commit()]
8. post_indexing()    ← AFTER commit, spawned as async task
```

**Security properties:**

- Pre-indexing runs before any state mutation -- indexer cannot influence tx execution.
- State root is computed before `index_block()` -- indexer cannot affect `app_hash`.
- Post-indexing is async and non-blocking -- indexer errors don't halt the chain.
- Pre-indexing and index_block errors are fatal (halt block processing).

### HookedIndexer (composition)

`indexer/hooked/` provides a composite pattern for chaining multiple indexers:

```rust
pub struct HookedIndexer {
    indexers: Arc<RwLock<Vec<Box<dyn Indexer + Send + Sync>>>>,
}
```

Indexers are added in order and each receives a shared `IndexerContext` (type-safe
key-value store using `http::Extensions`). Earlier indexers can produce data consumed
by later ones.

## 2. SQL Indexer (`indexer/sql/`)

### Schema

| Table          | Key Columns                                                   | Purpose         |
| -------------- | ------------------------------------------------------------- | --------------- |
| `blocks`       | `height` (unique), `hash`, `app_hash`                         | Block headers   |
| `transactions` | `hash` (unique), `block_height`, `sender`, `status`           | Tx metadata     |
| `messages`     | `transaction_id`, `contract_addr`, `method_name`              | Sub-tx messages |
| `events`       | `transaction_id`, `block_height`, `event_type`, `data` (JSON) | Emitted events  |

Indexes exist on `block_height`, `hash`, `sender`, `contract_addr`, and `events.data`
(JSON).

### HTTP request tracking

Each transaction records the HTTP peer that submitted it:

```rust
pub struct HttpRequestDetails {
    pub remote_ip: Option<String>,
    pub peer_ip: Option<String>,
    pub created_at: u64,
}
```

### Persistence properties

- **Idempotent:** `save_block()` checks if the block already exists before inserting
  (safe for crash recovery).
- **Atomic:** All table writes within a single database transaction.
- **Batch-safe:** Inserts batched to 2,048 rows to respect PostgreSQL argument limits.

### Event cache

An in-memory ring buffer of recent block events (`indexer/sql/src/event_cache.rs`).
Configurable window size. Used for fast GraphQL lookups without DB round-trips.

## 3. Cache Indexer (`indexer/cache/`)

Persists complete block + outcome data to disk for recovery:

```text
~/.dango/indexer/blocks/{height}.json  -- Serialized block data
~/.dango/indexer/last_block.json       -- Latest block height
```

Optional S3 sync with bitmap tracking of uploaded blocks.

## 4. Dango-Specific Indexer (`dango/indexer/`)

Extends the base SQL indexer with domain-specific data extraction:

```rust
// Runs in post_indexing (async, non-blocking)
let (transfers, accounts, perps) = tokio::join!(
    transfers::save_transfers(&ctx, block_height),
    accounts::save_accounts(&ctx, block, app_cfg),
    perps_events::save_perps_events(&ctx, block, app_cfg),
);
```

Extracts:

- **Account events:** `UserRegistered`, `AccountRegistered`, `KeyOwned`, `KeyDisowned`
  → accounts, users, public_keys tables.
- **Transfer events:** Bank transfer events → transfers table.
- **Perps events:** Trade execution, funding, liquidation → perps_events table.

Only processes committed events from successful transactions.

## 5. GraphQL / HTTP Server (`indexer/httpd/`)

Actix-web HTTP server with async-graphql:

| Parameter            | Value  |
| -------------------- | ------ |
| Workers              | 8      |
| Max connections      | 10,000 |
| Backlog              | 8,192  |
| Max blocking threads | 16     |

### Query types

- `block(height)` / `blocks(first, after)` -- Block headers with nested transactions
  and events.
- `transaction(hash)` / `transactions(first, after)` -- Tx metadata with nested
  messages and events.
- `events(filter)` -- Event queries with JSON data filtering.

### Subscriptions

Real-time subscriptions via PostgreSQL LISTEN/NOTIFY:

- `blockMinted` -- New blocks.
- `transactionProcessed` -- New transactions.
- `eventEmitted` -- New events.

### Data loaders

N+1 query prevention via dataloaders:

- `BlockTransactionsDataLoader`, `BlockEventsDataLoader`
- `TransactionEventsDataLoader`, `TransactionMessagesDataLoader`
- `EventTransactionDataLoader`, `FileTransactionDataLoader`

## 6. Node Startup (`dango/cli/`)

The `dango start` command initializes and runs the full node:

```text
1.  Parse CLI args and config
2.  Initialize telemetry (Sentry + OpenTelemetry)
3.  Initialize metrics (Prometheus)
4.  Open DiskDb (RocksDB)
5.  Create RustVm
6.  Create base App
7.  Setup indexer stack:
    ├── CacheIndexer (disk persistence)
    ├── SqlIndexer (PostgreSQL)
    ├── DangoIndexer (domain-specific extraction)
    └── ClickHouseIndexer (analytics)
8.  Run DB migrations + catch-up reindexing
9.  Spawn:
    ├── Dango HTTP server (GraphQL)
    ├── Metrics HTTP server (Prometheus)
    └── ABCI server (CometBFT connection)
10. Signal handlers (SIGINT, SIGTERM)
```

### ABCI server split

The app is split into four ABCI service components:

- **Consensus:** FinalizeBlock, Commit
- **Mempool:** CheckTx
- **Snapshot:** State sync
- **Info:** Query, simulation

### Graceful shutdown

1. Set HTTP shutdown flags (return 503 for new requests).
2. Wait 100ms for propagation.
3. Shutdown indexer (wait for async tasks to complete).
4. Flush telemetry (Sentry, OpenTelemetry).

## 7. Security Analysis

### Trust boundaries

```text
┌────────────────────────────────────────────────────────┐
│ Consensus-Critical (ABCI)                              │
│  App + DB + VM                                         │
│  Indexer trait called but read-only                    │
│  State root determined before indexer writes           │
└────────────────── ▼ ───────────────────────────────────┘
                    │ Block + Outcome
┌───────────────────┴────────────────────────────────────┐
│ Non-Consensus (Indexer Stack)                          │
│  CacheIndexer → disk                                   │
│  SqlIndexer → PostgreSQL                               │
│  DangoIndexer → domain tables                          │
│  ClickHouseIndexer → analytics DB                      │
└────────────────── ▼ ───────────────────────────────────┘
                    │
┌───────────────────┴────────────────────────────────────┐
│ Public API (GraphQL/HTTP)                              │
│  Read-only queries over indexed data                   │
│  No state mutation capability                          │
└────────────────────────────────────────────────────────┘
```

### Network exposure

| Component     | Default Port | Exposure                        |
| ------------- | ------------ | ------------------------------- |
| CometBFT RPC  | 26657        | Public (read-only)              |
| ABCI          | 26658        | Localhost only (CometBFT ↔ App) |
| Dango GraphQL | 8000         | Configurable                    |
| Metrics       | 8001         | Internal                        |
| PostgreSQL    | 5432         | Private                         |

### Known gaps

- **No GraphQL query complexity limits.** Deeply nested or expensive queries could DoS
  the HTTP server.
- **No HTTP rate limiting.** Any client can issue unlimited queries.
- **Event JSON size unbounded.** Malicious contracts could emit large events,
  inflating the database.
- **IP logging without TTL.** Transaction origin IPs stored indefinitely.
