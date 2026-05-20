# `RemoteBlockSource` — V2 design (placeholder)

Concrete implementation of the [`BlockSource`](../DESIGN.md#blocksource) trait
for the V2 deployment: the historical indexer runs on a different host from
the dango node (or after the in-process indexer is retired). Raw blocks are
no longer available on the local filesystem.

> **Not implemented in V1.** This document is a stub — it sketches where the
> design is going so that decisions in V1 don't accidentally close doors
> here. Fill it out when V2 is on the roadmap.

## Intent

A `RemoteBlockSource` owns its own storage of raw blocks (the dango node's
local cache is no longer reachable) and pulls those blocks from one or more
remote backends. Postgres is the natural store: idempotent inserts, sub-ms
point queries on `(height PK, payload BYTEA)`, gap detection via window
functions.

From the app's perspective, the source still exposes the same `BlockSource`
trait — `run`, `get`, `subscribe`, `contiguous_frontier`. The remote-ness is
entirely internal.

## Internal trait surface (sketch — not part of the public contract)

Internally the source will compose a few smaller pieces. These are
*not* exposed to the app; they live inside this module.

- A `RawBlockFetcher` trait — pull blocks in contiguous batches from a
  given backend. Concrete impls (illustrative):
  - `HttpdFetcher` — REST against a sentinel `dango-httpd`.
  - `B2ChunkFetcher` — zstd-dict-compressed chunks on B2.
  - `LayeredFetcher` — compose multiple backends (recent → sentinel,
    old → B2).
- A `LiveSubscriber` (or equivalent) — push notifications of new blocks
  from a sentinel `dango-httpd`. The exact wire protocol (WS / SSE / gRPC)
  is TBD.
- A Postgres store — owns the `blocks_raw(height PK, payload BYTEA)` table
  and the gap-detection logic.

The source coordinates them: subscriber feeds the live tail, fetcher
backfills the gaps below the live tail, store records and serves.

## Storage cost

Raw blocks accumulate for the lifetime of the indexer. Estimates:

| Period | Raw zstd-dict | Notes |
|---|---|---|
| Today (~22M blocks) | ~210 GB | |
| 1 year | ~600 GB | |
| 5 years | ~2 TB | manageable on enterprise SSD |

When this becomes a real concern, an optional GC kicks in: when all
registered projections are past height H, drop blocks below H. New
projections deployed after GC fetch missing blocks transparently via the
fetcher → B2. Implementation detail of the source, no trait change to the
public surface.

## Open questions (V2-specific, to revisit when implementing)

- Wire protocol for the live subscriber: WS / SSE / gRPC. Each has
  trade-offs on browser-friendliness, backpressure, reconnection semantics.
- Error model for the fetcher: structured errors (e.g.
  `NotAvailable { earliest: u64 }`) vs. plain REST with 404s.
- Sentinel-side API contract — what `dango-httpd` actually exposes to
  remote clients. To be designed when V2 lands.
- Whether the Postgres `blocks_raw` table should be partitioned by height
  range (e.g. every 10M blocks) up front, to make future GC a partition
  drop instead of a row delete.
- Schema versioning on the raw payload: include a `u16 schema_version`
  field early to avoid painful migrations later on 22M+ rows.
