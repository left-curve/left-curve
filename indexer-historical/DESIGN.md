# Historical Indexer: Design

Internal design of the standalone `indexer-historical` binary — the
external service that consumes finalized blocks from the dango chain and
exposes structured data (Postgres / ClickHouse / GraphQL / REST) to clients.

In this document "indexer" refers exclusively to this binary — separated
from the dango chain binary, replacing today's in-process `indexer/sql/` +
`indexer/clickhouse/` crates as the place where structured indexing lives.

## Crate layout

Split into five crates, each with a focused responsibility. The split is
deliberately granular: it keeps each crate small enough to stay flat
(`no sub-directories in src/`, per the project's rust guidelines) and
makes the dependency graph explicit at the workspace level.

```
indexer-historical/
├── types/         → lib `indexer-historical-types`
│                     • shared wire types (BlockData, ...)
│                     • no deps on other historical crates
├── block-source/  → lib `indexer-historical-block-source`
│                     • trait BlockSource
│                     • concrete impls (LocalBlockSource, ...)
│                     • deps: types
├── projection/    → lib `indexer-historical-projection`
│                     • trait Projection
│                     • deps: types
├── app/           → lib `indexer-historical-app`
│                     • App struct + projection_loop + Config
│                     • DB migrations, GraphQL resolvers (where applicable)
│                     • deps: types, block-source, projection
└── cli/           → bin `indexer-historical`
                     • clap subcommands (init, start, drop-*, ...)
                     • config parsing (file + env)
                     • telemetry/signal setup
                     • delegates to app
                     • deps: app
```

Dependency graph (acyclic):

- `types` is the foundation: no dependencies on any other historical crate.
- `block-source` depends on `types`.
- `projection` depends on `types`.
- `app` depends on `types`, `block-source`, and `projection`.
- `cli` depends on `app`.

```
              cli
               │
               ▼
              app
              ╱│╲
            ╱  │  ╲
          ╱    │    ╲
block-source   │   projection
          ╲    │    ╱
            ╲  │  ╱
              ╲│╱
              types
```

Concrete `BlockSource` implementations are documented separately:

- [`LocalBlockSource` (V1)](./design/local-block-source.md) — co-located
  with the dango node.
- [`RemoteBlockSource` (V2, placeholder)](./design/remote-block-source.md)
  — detached deployment, owns its own raw store.

## Wire payload

```rust
/// What flows through the source. No HTTP/sentinel metadata, only chain data.
pub struct BlockData {
    pub block: Block,
    pub outcome: BlockOutcome,
}
```

Wrapping `(Block, BlockOutcome)` in a nominal type lets us add carrier
metadata later (chunk_id, schema version, integrity hash) without
changing trait signatures.

## Architecture overview

Two cooperating layers visible to the app:

```
                  ┌────────────────────────┐
                  │  BlockSource           │
                  │  • feeds new blocks    │
                  │  • tracks frontier     │
                  │  • broadcasts          │
                  │  • serves get(h)       │
                  └────────────┬───────────┘
                               │ subscribe() / get(h)
              ┌────────────────┼─────────────────┐
              ▼                ▼                 ▼
         ┌──────────┐    ┌──────────┐      ┌──────────┐
         │Projection│    │Projection│      │Projection│
         │"candles" │    │"tx_user" │      │"deposits"│
         │ → CH     │    │ → PG     │      │ → PG     │
         └──────────┘    └──────────┘      └──────────┘
```

- **`BlockSource`** is the single abstraction the app talks to. It hides
  how blocks are obtained (GraphQL subscription, S3, B2, ...) and where
  raw blocks are stored (in the source itself, on the node's disk, ...).
- **`Projection`s** are independent consumers — each owns its tables, its
  own watermark, its own backend (PG or CH). They build the user-facing
  tables.

The source does **not** explode blocks into domain tables. That is the
projections' job.

## Traits

### `BlockSource`

```rust
#[async_trait]
pub trait BlockSource: Send + Sync {
    /// Start the source's internal tasks (subscribe, fetch, ...).
    /// Returns when the source has terminated (clean shutdown or error).
    async fn run(self: Arc<Self>) -> AnyResult<()>;

    /// Read one block by height. Used by projections during catch-up.
    /// Implementations may serve this from local storage, from a remote
    /// backend, or from the on-disk cache of the co-located node.
    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>>;

    /// Subscribe to the live stream of newly-contiguous blocks. Multi-
    /// subscriber via tokio broadcast; payload is `Arc<BlockData>` so all
    /// projections share the same in-memory copy.
    fn subscribe(&self) -> broadcast::Receiver<Arc<BlockData>>;

    /// Highest H such that all heights in [min..H] are reachable through
    /// this source. Projections use this to know when catch-up is done
    /// and they can rely on the broadcast for the live tail.
    async fn contiguous_frontier(&self) -> AnyResult<Option<u64>>;
}
```

Concrete implementations:

- [`LocalBlockSource`](./design/local-block-source.md) — V1. Subscribes to
  the in-process `dango-httpd` GraphQL `block` subscription for live
  notifications, reads payloads from the node's existing cache files on
  disk. No storage of its own.
- [`RemoteBlockSource`](./design/remote-block-source.md) — V2 placeholder.
  Owns a Postgres `blocks_raw` table; internally composes a fetcher
  (httpd / B2 / layered) and a live subscriber.

### `Projection`

A projection is a self-contained indexer over the block stream. Each
projection owns its tables, its watermark, and (optionally) its backend.
Adding a new projection later means: pick a stable `id`, deploy it, it
backfills from `min_height` while existing projections keep running.

```rust
#[async_trait]
pub trait Projection: Send + Sync {
    /// Stable id used to persist this projection's watermark. Bumping the
    /// id forces a full re-backfill (new id ⇒ empty watermark).
    fn id(&self) -> &'static str;

    /// Minimum height below which this projection has nothing to do
    /// (e.g. a contract that didn't exist before that block).
    fn min_height(&self) -> u64 { 0 }

    /// Process one block. Writes to this projection's own tables AND
    /// updates the projection's watermark in the same transaction (where
    /// the backend supports it — see open questions).
    async fn process(&self, block: &BlockData) -> AnyResult<()>;

    /// Last height fully processed by this projection. None if not started.
    async fn last_processed_height(&self) -> AnyResult<Option<u64>>;
}
```

Concrete implementations live in the app crate, one per domain area:
`CandlesProjection` (→ ClickHouse), `TxPerUserProjection` (→ Postgres),
`PerpsTradesProjection` (→ ClickHouse), `GatewayDepositsProjection` (→
Postgres), etc.

## Why raw blocks are reachable from the source

Wherever the raw block lives (inside the source's own store, or on the
node's disk under V1), keeping it reachable through `source.get(h)` buys
two things:

1. **Multiple projections from one source of truth.** A new domain table
   added six months in (e.g. "perp fees per user") needs to backfill from
   height 0. The new projection reads `source.get(h)` and processes — no
   second pass to the chain or to B2 needed.

2. **Schema agility.** A projection can be dropped and rebuilt: bump its
   `id`, deploy, watch it catch up. The raw block stays put. This replaces
   today's "stop the indexer, drop tables, restart, reindex everything"
   cycle with per-projection isolated rebuilds.

## App run loop

```rust
struct App {
    source: Arc<dyn BlockSource>,
    projections: Vec<Arc<dyn Projection>>,
}

impl App {
    async fn run(&self) -> Result<()> {
        let mut handles = vec![spawn(self.source.clone().run())];
        for p in &self.projections {
            handles.push(spawn(projection_loop(
                p.clone(),
                self.source.clone(),
            )));
        }
        try_join_all(handles).await?;
        Ok(())
    }
}
```

Two kinds of task: the source's internal `run` (whatever it does inside —
WS subscriptions, fetchers, store writes), and one `projection_loop` per
projection. The app doesn't orchestrate anything else — in particular, it
does not subscribe to the broadcast on behalf of the projections; each
`projection_loop` subscribes itself, lazily, after its first pull catch-up.

### Projection loop

Each projection alternates between two phases:

1. **Pull catch-up** — read via `source.get(cursor)` and process, one
   block at a time. Exits when the source returns `None` (the projection
   is at the source's current frontier, possibly with gaps still to fill
   in `RemoteBlockSource` setups).
2. **Push live** — sit on the broadcast and consume new blocks as they
   arrive. Inner loop stays here until the broadcast says we need to
   catch up again.

The subscribe is **lazy**: the receiver is created after the first Phase 1
exits. This matters when a fresh projection backfills from height 0 with
a chain already millions of blocks ahead — we don't want the broadcast
buffer to fill with messages we're about to process via `get()` anyway.
On subsequent iterations the receiver is reused.

The same loop handles both phases, including transitions:

```rust
async fn projection_loop(
    p: Arc<dyn Projection>,
    source: Arc<dyn BlockSource>,
) -> Result<()> {
    let mut cursor = p
        .last_processed_height()
        .await?
        .map(|h| h + 1)
        .unwrap_or_else(|| p.min_height());

    let mut maybe_rx = None;

    loop {
        // PHASE 1 — catch-up via pull
        while let Some(block) = source.get(cursor).await? {
            p.process(&block).await?;
            cursor += 1;
        }

        let rx = maybe_rx.get_or_insert_with(|| source.subscribe());

        // PHASE 2 — live via push
        loop {
            match rx.recv().await {
                Ok(block) => match block.block.info.height.cmp(&cursor) {
                    // Behind cursor: already processed via Phase 1
                    // catch-up; drain it from the buffer and keep going.
                    Ordering::Less => continue,
                    Ordering::Equal => {
                        p.process(&block).await?;
                        cursor += 1;
                    }
                    // Ahead cursor: there's a gap between cursor and the
                    // block we just got. Phase 1 catch-up via `get()` is
                    // the right tool — break and retry.
                    Ordering::Greater => break,
                },
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Same recovery path as Greater.
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    bail!("source broadcast closed unexpectedly");
                }
            }
        }
    }
}
```

Projections never poll on a timer. The only sleep happens implicitly
inside `rx.recv().await` when there's nothing to do.

### BlockSource invariants the loop relies on

For the loop above to be correct on any `BlockSource` impl (V1, V2, or
test mocks), the source must guarantee:

1. **`contiguous_frontier()` is monotonic** — never decreases.
2. **`h <= contiguous_frontier ⟹ get(h) == Some`** — within the
   contiguous prefix, blocks are always reachable through `get`.
3. **Broadcast emits in strict +1 order** — every `broadcast.send(block)`
   happens immediately after a `contiguous_frontier.store(h)` with
   `block.height == h`. No skipping, no out-of-order.
4. **`get(h) == None` is "not yet", not "definitively absent"** — the
   source must not GC a height that some projection might still try to
   reach via `get`.

A source that violates any of these is broken; the loop is intentionally
not defensive against them.

## Projection backends

Projections are **free to choose** their own backend per-domain. A
projection is just a `process(block)` function that writes wherever it
wants. Typical mapping:

| Projection (example) | Backend | Why |
|---|---|---|
| `candles_*` (OHLC) | ClickHouse | time-series, aggregations, columnar |
| `perps_trades_agg` | ClickHouse | bulk inserts, group by |
| `tx_per_user` | Postgres | point/range query per address |
| `events_per_user` | Postgres | range scan + filters |
| `gateway_deposits` | Postgres | few rows, OLTP queries |
| `leaderboard_volumes` | ClickHouse | massive aggregation |

OLTP-shaped GraphQL queries hit PG, OLAP-shaped queries hit CH —
unchanged from today's split, just reorganized as projections.

## Deployment scenarios

Two target setups, picked at startup by config. Both implement the same
`BlockSource` trait — the projections and the app loop don't change
between them.

| Setup | Source impl | Notes |
|---|---|---|
| **V1** — Co-located | [`LocalBlockSource`](./design/local-block-source.md) | Reuses the dango node's GraphQL `block` subscription + cache files on disk. No new storage. |
| **V2** — Detached | [`RemoteBlockSource`](./design/remote-block-source.md) | Owns its own Postgres `blocks_raw` store. Internally composes a fetcher (httpd / B2) and a live subscriber. **Not in V1.** |

Switching from V1 to V2 is a config change and (for V2) a fresh DB. The
projections themselves are unchanged.

## Configuration

Surfaced through the CLI's config file (+ env overrides):

| Field | Default | Description |
|---|---|---|
| `source` | `local` | Which `BlockSource` impl to instantiate. |
| `pubsub_buffer_size` | 10_000 | Broadcast channel capacity. At 2 bps, ~83 min of buffer before a slow projection is `Lagged`. |

Source-specific options (`local.*`, `remote.*`) live in nested sections
and are documented in the corresponding sub-spec.

## Operational flow (V1, worked example)

```
t=0   Chain at 1.0M. Indexer fresh.
      → LocalBlockSource boots: queries dango-httpd for latest indexed
        block → 1.0M.
      → contiguous_frontier = 1.0M.
      → WS subscribe to dango-httpd `block` opens.
      → Projections boot with cursor = their last_processed (or min_height).

t=k₁  Projections still in catch-up via source.get(h), reading files
      from disk one at a time. Frontier unchanged at 1.0M.

t=k₂  New block 1.0M+1 finalized by node. Subscription notifies.
      Source reads file, advances frontier → 1.0M+1, broadcasts.
      Projections still in catch-up don't see this directly; when they
      reach the frontier, they hit Phase 2 and pick up via broadcast.

t=k₃  All projections at the frontier, sitting on rx.recv().

t=k₄  Indexer crashes. Restart.
      Source re-queries dango-httpd → latest indexed block is now 1.2M.
      contiguous_frontier = 1.2M.
      WS reconnects, starts streaming from 1.2M+1.
      Projections resume from their last_processed → catch-up via get
      until they reach 1.2M, then push-live.
```

## What this design does NOT cover

Out of scope here, documented elsewhere or to be designed later:

- **`LocalBlockSource` internals** — see
  [local-block-source.md](./design/local-block-source.md).
- **`RemoteBlockSource` internals** — see
  [remote-block-source.md](./design/remote-block-source.md).
- **Projection schemas**: concrete table designs for candles, tx_per_user,
  events_per_user, perps_trades, etc. One document per projection or
  grouped, TBD.
- **GraphQL/REST front door**: query layer over the projected data.
  Carries over largely unchanged from today's `indexer/httpd/`.

## Open questions

- **Projection atomicity on ClickHouse**: the trait says `process` writes
  tables and watermark together. PG supports a real transaction;
  ClickHouse doesn't have multi-statement transactions in the PG sense.
  Concrete options: watermark derived from `max(height)` in the data
  table; idempotent inserts via `ReplacingMergeTree` so re-processing is
  a no-op; watermark in PG with best-effort coupling. Decide before the
  first CH projection lands.
- **Convenience supertraits** (`PostgresProjection`, `ClickhouseProjection`)
  to factor out connection pooling + watermark boilerplate?
- **`reindex` CLI subcommand**: drop a projection's tables + watermark
  and let it rebuild from the source. Useful enough to ship in V1, or
  defer?
- **Schema version on `BlockData`**: should the wire payload include a
  `u16 schema_version` from day 1? Cheap insurance against future binary
  layout changes.
- **Observability**: every projection should expose `lag_seconds` and
  `last_processed_height` as metrics; the source should expose
  `frontier`. List the minimum set as part of the V1 cut.
- **Testing strategy**: with everything behind two traits, a `MemoryBlockSource`
  is enough to unit-test the app + projection loops. Add a section.
