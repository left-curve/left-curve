# Historical Indexer: Observability

How the `indexer-historical` service exposes its internal state — **metrics**
(Prometheus pull) and **structured logs** (per-component `tracing` spans) — so an
operator can answer, from Grafana and Loki alone:

- *Is each projection keeping up, and at what height?*
- *Where is the block source spending time — backfill, live tail, store writes?*
- *Which queue is the bottleneck?*
- *Is RocksDB healthy (compaction backlog, write stalls, cache pressure)?*
- *Which read queries are slow?*

This resolves the **Observability** open question in [`../DESIGN.md`](../DESIGN.md).

The intent (the *roles* below) is fixed; the concrete metric names, label sets,
and the 5 s sample cadence are an implementation that can be tuned without
changing what is observed.

## Conventions

- **Names**: flat `snake_case`, prefix `indexer_historical_`, matching the two
  counters that already shipped (`indexer_historical_activity_*_total`).
  Counters end `_total`; latency histograms end `_duration_seconds` (seconds, as
  `Instant::elapsed().as_secs_f64()`); gauges are bare nouns.
- **Exception — RocksDB internals** reuse dango's exact names (`rocksdb_*` with
  `type` / `cf` labels, §RocksDB internals), so the **node's RocksDB Grafana
  dashboard works verbatim** against this service's `/metrics` (separate process,
  separate scrape target — no collision). Everything else is `indexer_historical_*`.
- **Gating**: every metric is `#[cfg(feature = "metrics")]`, every span /
  event `#[cfg(feature = "tracing")]`; both features are default-on (per
  `.claude/rules/rust.md`). With a feature off the macros compile out to nothing.
- **Histograms render as Prometheus summaries** (rolling quantiles) — the repo
  sets no custom buckets, the established convention. Rates/lags are derived in
  Grafana (`rate()`, `frontier − height`), as the user expects.
- **Description**: each crate ships an always-present `init_metrics()` whose body
  is metrics-gated (`describe_*!` help strings, `Once`-guarded), called once from
  the cli at boot — the dango `metrics.rs` idiom.

## Wiring

- The cli installs the global recorder first thing in `start`
  (`PrometheusBuilder::new().install_recorder()` → `PrometheusHandle`) and calls
  every crate's `init_metrics()`. Recording is then always on (cheap, NoopRecorder
  otherwise), independent of whether the endpoint is served.
- A `[metrics]` config section (`enabled` / `ip` / `port`, default
  `true` / `0.0.0.0` / `9191`, env-overridable like everything else) controls the
  scrape endpoint. When enabled, the cli serves `GET /metrics` via the shared
  `dango_indexer_metrics::run_metrics_server` (the same helper the node uses), on
  its own thread with its own actix `System` — the proven pattern from the
  read-API `serve`. It is supervised at the top level alongside `App::run`, so it
  stays up even in ingest-only mode (`httpd.enabled = false`).

## Spans (logs)

Every long-lived component runs inside its own span, so every event it emits —
and every event from the code it calls — carries the component name; filterable
in Loki by `span`.

| Span | Where | Fields |
|---|---|---|
| `projection` | each `projection_loop` task (app) | `id` (= projection id) |
| `process` | `Projection::process` (nests under `projection`) | `height` |
| `bsource.coordinator` | remote: persist→broadcast loop | — |
| `bsource.live` | remote: live-tail subscriber loop | — |
| `bsource.healer` | remote: gap-repair loop | — |
| `bsource.backfill` | remote: one gap fill | `from`, `to` |
| `bsource.fetcher` | sentinel range-fetch task | `from`, `to` |
| `bsource.sampler` | remote: periodic metric sampler | — |
| `bsource.local` | local source run loop | — |

A log line tagged `projection{id=activity}` or `bsource.healer` tells you *who*
emitted it without reading the message — the user's explicit ask.

## Metric catalog

### Block source — frontier & progress (local + remote)

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_block_source_frontier` | gauge | — | highest contiguous height reachable via `get` — the source's sync anchor |
| `indexer_historical_block_source_live_height` | gauge | — | highest height seen on the live subscription (the observed chain tip) |
| `indexer_historical_block_source_live_blocks_total` | counter | — | blocks received from the live subscription |
| `indexer_historical_block_source_broadcast_sent_total` | counter | — | frontier-advance broadcasts emitted to projections |
| `indexer_historical_block_source_reconnects_total` | counter | `reason` ∈ subscribe_failed/stream_error/stream_ended | live re-subscribes |
| `indexer_historical_block_source_discontinuities_total` | counter | — | live-tail holes detected (`height > prev+1`) |

`live_height − frontier` ⇒ blocks still to backfill below the tip.

### Block source — healer / backfill (remote)

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_block_source_healing` | gauge 0/1 | — | a gap backfill is in progress |
| `indexer_historical_block_source_gap_low` | gauge | — | lowest missing height (gap `from`); 0 when gap-free |
| `indexer_historical_block_source_gap_high` | gauge | — | top of the lowest gap (`to`); 0 when gap-free |
| `indexer_historical_block_source_backfill_failures_total` | counter | — | gap contiguity-check failures (misbehaving fetcher) |

### Fetcher — backfill throughput (sentinel)

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_block_fetcher_blocks_total` | counter | — | blocks delivered by the fetcher — `rate()` ⇒ **recovery speed** |
| `indexer_historical_block_fetcher_requests_total` | counter | `outcome` ∈ ok/error/timeout/empty | `/block/full/range` calls |
| `indexer_historical_block_fetcher_request_duration_seconds` | histogram | — | per range HTTP call |

### Channels — fullness (bottleneck finder)

`depth / capacity` per channel locates the backpressure point: a full **fetcher**
queue ⇒ the store writer is the bottleneck; a full **coordinator** queue ⇒ the
store can't keep up with both writers; a full **broadcast** ⇒ a projection lags.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_channel_depth` | gauge | `channel` ∈ broadcast/coordinator/fetcher | queued items |
| `indexer_historical_channel_capacity` | gauge | `channel` | configured buffer size (constant) |
| `indexer_historical_channel_receivers` | gauge | `channel="broadcast"` | live broadcast subscribers |

### Block store — operation latency (RocksDB)

| Metric | Type | Meaning |
|---|---|---|
| `indexer_historical_block_store_put_duration_seconds` | histogram | persist + checkpoint one block (encode + `WriteBatch` write) — **commit time** |
| `indexer_historical_block_store_get_duration_seconds` | histogram | point read + borsh decode — **block read time** |
| `indexer_historical_block_store_blocks_persisted_total` | counter | blocks written to the store |

### RocksDB internals (mirror dango — identical names for dashboard reuse)

Sampled every 5 s by the store's metric hook, per column family (`cf` ∈
`default` / `blocks`), `type` selecting the sub-metric — same shape as the node's
`StatisticsWorker`:

`rocksdb_memtable_bytes`, `rocksdb_sst_bytes`, `rocksdb_compaction_bytes`,
`rocksdb_block_cache_bytes`, `rocksdb_blob_bytes` (new — the BlobDB payload
files, ~95 % of bytes here), `rocksdb_memtable_count`, `rocksdb_compaction_count`,
`rocksdb_lsm_count` (incl. `num_files_at_level{level}`), `rocksdb_errors_count`,
`rocksdb_flags` (`is_write_stopped`, `compaction_pending`, …).

Integer properties only (no overhead); the seek-latency p95 gauge is omitted (it
needs RocksDB statistics enabled) and can be added if a read-latency question
arises.

### Projection — sync state

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_projection_height` | gauge | `projection` | last committed height (from the committer — one place sees every cursor) |
| `indexer_historical_projection_blocks_total` | counter | `projection` | blocks committed |
| `indexer_historical_projection_process_duration_seconds` | histogram | `projection` | time staging one block (`process`) |
| `indexer_historical_projection_commit_duration_seconds` | histogram | `projection` | time committing (CH flush + PG tx) |
| `indexer_historical_projection_lagged_total` | counter | `projection` | broadcast-overflow → Phase-1 fallbacks (broadcast under-sized) |

Per-projection lag = `block_source_frontier − projection_height{projection}`,
derived in Grafana. Catch-up speed = `rate(projection_blocks_total)`.

### Activity projection — write volume

`indexer_historical_activity_transactions_total`,
`indexer_historical_activity_events_total`,
`indexer_historical_activity_event_data_total` (counters; rows staged per table).

### Read queries — latency

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_query_duration_seconds` | histogram | `query` | per-feed DB query latency (the hand-written SQL) |
| `indexer_historical_query_total` | counter | `query`, `outcome` ∈ ok/error | per-feed executions |
| `indexer_historical_http_request_duration_seconds` | histogram | — | end-to-end HTTP request handling |
| `indexer_historical_http_requests_total` | counter | — | HTTP requests served |
| `indexer_historical_http_in_flight` | gauge | — | concurrent in-flight HTTP requests |

`query` values: `events_by_type`, `contract_events`, `events_involving`,
`contract_events_involving`, `transactions_involving`, `transactions_by_hash`.
A feed slow relative to the others points straight at an index miss. (The core
`GET /block/{height}` route is not a feed; its latency rides the end-to-end HTTP
histogram below.)

### Misc

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `indexer_historical_block_loader_failures_total` | counter | — | block hydrations that errored (served as "unavailable") |
| `indexer_historical_build_info` | gauge=1 | `version`, `commit` | deployed build |

## Dashboard cheat-sheet

- **Per-projection lag**: `indexer_historical_block_source_frontier - indexer_historical_projection_height`
- **Backfill speed**: `rate(indexer_historical_block_fetcher_blocks_total[1m])`
- **Live ingest rate**: `rate(indexer_historical_block_source_live_blocks_total[1m])`
- **Bottleneck**: `indexer_historical_channel_depth / indexer_historical_channel_capacity`
- **Store write health**: `rocksdb_flags{type="is_write_stopped"}`,
  `rocksdb_compaction_bytes`, `indexer_historical_block_store_put_duration_seconds`
- **Slow query**: `indexer_historical_query_duration_seconds` by `query`
</content>
</invoke>
