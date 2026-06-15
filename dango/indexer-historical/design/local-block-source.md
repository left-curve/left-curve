# `LocalBlockSource` — V1 design

Concrete implementation of the [`BlockSource`](../DESIGN.md#blocksource) trait
for the V1 deployment: the historical indexer runs on the same host as the
dango node and reuses the data the node already persists.

> See [DESIGN.md](../DESIGN.md) for the shared contract (`BlockSource`,
> `Projection`, app loop). This document covers only V1-specific concretions.

## Why this impl

The dango node, through its in-process indexer (`indexer/sql/`,
`indexer/clickhouse/`), already writes every finalized block to disk and
exposes a GraphQL subscription for new-block notifications. V1 piggy-backs
on both — no duplicate storage, no new wire protocol.

## Inputs the source relies on

1. **GraphQL subscription** at `dango-httpd` — `block { block_height }`. Notifies
   when a new block has been indexed and is reachable on disk.
2. **Cache files on disk** — written by the node's `dango-indexer-cache` crate at:
   ```
   <dir>/blocks/<last-3-digits-of-height-reversed>/<height>.borsh[.xz]
   ```
   Each file is a borsh-serialized
   `CacheFile { data: BlockAndBlockOutcomeWithHttpDetails { block, block_outcome, http_request_details }, .. }`,
   optionally lzma-compressed (`.xz`). The historical indexer drops
   `http_request_details` after deserialization — the wire `BlockData` only
   carries `block` + `block_outcome`.

Both inputs come from the same dango node process. If the node is down,
both are down — single failure domain, no consistency issues to worry about.

## Live tail — GraphQL subscription, not file watching

The subscriber half of the source opens a WebSocket to `dango-httpd` and
subscribes to `block { block_height }`. For each event it loads the
corresponding cache file from disk, decodes it into a `BlockData`, advances
the frontier, and broadcasts.

Why GraphQL subscription over polling or inotify:

- **API contractual** — depends on the GraphQL schema (stable, versioned)
  instead of the on-disk file layout.
- **Reconnection** is solved by the WS client; no custom retry loop.
- **"Live from tip"** semantics come for free — the subscription naturally
  starts at the chain head and moves forward.
- **No file enumeration** to discover what appeared.
- **Forward-compatible** with V2: same subscriber code can connect to a
  remote `dango-httpd` later, only the URL changes.

Notification-only: the subscription doesn't carry the full `Block + BlockOutcome`
payload. The source still needs to read from disk to assemble `BlockData`.
This is fine — the file is local and almost always in the OS page cache (the
node just wrote it).

## Catch-up — disk read on demand

When a projection lags behind the frontier, it calls `source.get(h)`. In V1
that's a single-file read:

```
path = <dir>/blocks/<height % 1000 reversed>/<height>.borsh[.xz]
load CacheFile via disk_saver::DiskPersistence
drop http_request_details
return BlockData
```

`DiskPersistence` already handles both compressed (`.xz`) and uncompressed
files transparently — see `utils/disk-saver/src/persistence.rs`.

No batching in V1: the projection loop reads one block at a time. A future
optimization could expose `get_range(from, max_count)` if profiling shows
disk read overhead matters; for now, page cache hides it.

## Frontier — queried at boot, advanced at runtime

The source holds `contiguous_frontier: AtomicU64` in memory.

**At boot**: a single GraphQL query against `dango-httpd` asks for the
latest indexed block height (`latestBlock { blockHeight }` or equivalent —
the entity helper `indexer_sql::entity::blocks::latest_block_height`
already exists; whether it's already wired to a GraphQL query field or
needs to be is an implementation detail). Frontier ← that height. Sub-ms,
one round-trip.

**At runtime**: on every new-block notification from the GraphQL
subscription, if the new height is `frontier + 1`, advance the frontier
and broadcast. If it's ≤ frontier (already seen), skip. If it skips ahead
(should never happen in V1, since the node writes blocks strictly in
order), hold and log a warning.

### Why query the indexer instead of walking the disk

The cache files on disk are written by the node *before* `indexer-sql`
indexes them on Postgres. So the indexer's "latest indexed" is up to
1–2 blocks behind the highest file on disk. That's harmless:

- Projections in catch-up call `source.get(h)`, which reads from disk
  directly — they see files above the frontier without trouble.
- Notifications for heights ≤ frontier are filtered (not broadcast) by
  the `h == frontier + 1` check, so no duplicate processing.
- The frontier is monotonically increasing; the small initial offset is
  absorbed as soon as the next `frontier + 1` notification arrives.

Querying the indexer is strictly faster than walking 22M files at boot,
and explicitly defers to the in-process indexer as the source of truth
for "what's indexed" — which is what a projection consumer cares about.

### Alternative considered: reuse the subscription's first event

The existing `block` GraphQL subscription emits the latest indexed block
as its first event (via `once(last_block)` — see
`indexer/httpd/src/graphql/subscription/block.rs`). We could skip the
separate query and initialize the frontier from that first event.

Rejected for V1: an implicit "the first event is special" invariant is
harder to read than a deliberate query. Reconsider if the extra
round-trip ever matters.

## Internal architecture (informal)

```
                  ┌──────────────────────────────────┐
                  │ LocalBlockSource                 │
                  │                                  │
   dango-httpd ──→│  WS sub task:                    │
   (GraphQL sub)  │    recv height                   │
                  │      → read file from disk       │
                  │      → advance frontier          │
                  │      → broadcast Arc<BlockData>  │
                  │                                  │
                  │  get(h) call:                    │
                  │    → read file from disk         │
                  │    → return BlockData            │
                  │                                  │
                  │  subscribe():                    │
                  │    → broadcast::Receiver         │
                  └──────────────────────────────────┘
```

One internal task (the GraphQL subscriber loop). `get` is synchronous from
the source's point of view — no shared state beyond the frontier atomic and
the broadcast sender.

## Failure modes

| Failure | Behavior |
|---|---|
| WS connection drops | GraphQL client reconnects internally; on reconnect the sub yields from the new tip. Any block missed during downtime is still on disk and reachable via `get` for projections in catch-up. The frontier may stall briefly. |
| `dango-httpd` down but node up | Same as WS drop — subscriber retries. Files on disk keep growing but the source doesn't see them until httpd comes back. Frontier stalls. Projections keep processing up to the old frontier. |
| Cache file missing for a notified height | Shouldn't happen — the node writes the file before publishing on pubsub. If it does (race, corruption), log + skip + leave frontier unchanged; next notification will retry. |
| File present but corrupt | `DiskPersistence::load` returns a borsh error. Bubbled up via `anyhow`. Operational decision (skip vs halt) deferred — TBD. |

## Coupling with `dango-indexer-cache`

The source depends on the concrete shape of `CacheFile` and
`BlockAndBlockOutcomeWithHttpDetails` in the `dango-indexer-cache` crate. Any
layout change in that crate is a breaking change for the historical indexer.

Since both crates live in the same monorepo, this is a compile-time
invariant: a refactor in `dango-indexer-cache` either updates the source or
fails to build.

## Open questions

- Should we expose `get_range(from, max_count)` on `BlockSource` to let
  projections in catch-up batch disk reads? Marginal until profiled.
- Behavior on corrupt-file detection: skip + alert vs halt. Probably skip
  in V1 (the in-process indexer would also catch it) but needs a config knob.
- Reconnection backoff for the GraphQL subscription: rely on the client
  library defaults, or override? Defer to implementation.
- `dango-httpd` startup ordering: if the historical indexer starts before
  `dango-httpd` is reachable, the source should retry rather than fail
  hard. Standard pattern, but make sure it's exercised in tests.
