# `RemoteBlockSource` — design

Concrete implementation of the [`BlockSource`](../DESIGN.md#blocksource) trait
for the **detached** deployment: the historical indexer runs on a host where
**no dango node runs**, so raw blocks are not available on the local
filesystem and the `LocalBlockSource` strategy (read the node's cache files
off disk) does not apply.

> See [DESIGN.md](../DESIGN.md) for the shared contract (`BlockSource`,
> `Projection`, app loop) and [local-block-source.md](./local-block-source.md)
> for the co-located V1 strategy. This document covers only the
> `RemoteBlockSource` concretions. Known gaps and review findings for the
> current in-progress implementation are tracked in
> [remote-block-source-known-issues.md](./remote-block-source-known-issues.md).

## Scope: what we build now vs. the end goal

The **end goal** (agreed with the team, *not* built now but kept in view): a
set of sentinel nodes upload finalized blocks to an object store (B2) as
compressed chunks while keeping a hot window (24–48h) of recent blocks. The
`RemoteBlockSource` then pulls cold history from B2 and recent blocks from a
sentinel. This is the tiered vision in `~/specs/left-curve/`
(`indexer-storage-architecture.md`, `indexer-roles-and-responsibilities.md`).

The **step we build now**: deploy the historical indexer on a node-less
server and have it pull *all* blocks from a sentinel node (which today still
holds the entire history). No B2, no chunking. This is enough to validate the
whole `RemoteBlockSource` shape — own storage, remote fetch, gap handling,
live tail — without the cold-archive machinery. The B2 backend slots in later
behind the same fetcher trait (see [End goal](#end-goal-b2-layered-fetcher)).

## Why this impl

With no co-located node, two things change relative to V1:

1. **The source must own its storage of raw blocks.** The node's on-disk
   cache is unreachable. Postgres is the natural store: idempotent inserts,
   sub-ms point queries on `(height PK, payload BYTEA)`, gap detection via
   window functions.
2. **The source must fetch blocks over the network.** Both the historical
   backfill (from genesis) and the live tail come from a remote sentinel.

From the app's perspective the source still exposes the same four-method
`BlockSource` trait. The remote-ness is entirely internal.

## Architecture overview

Four internal pieces, none exposed to the app:

```
RemoteBlockSource (impl BlockSource)
 ├── BlockStore (PG blocks_raw)              [concrete]
 │     put(h,data) idempotent · get(h) · gaps([min..L)) · max_contiguous()
 ├── LiveSubscriber (subscription to a sentinel) [concrete]
 │     yields L at startup, then writes the tail [L..∞) to the store
 ├── Arc<dyn BlockFetcher>                       [trait, ≥1 impl]
 │     spawn(from, to) -> FetchStream   (bounded backfill, dies at `to`)
 │       ├── SentinelBlockFetcher   ← built now
 │       └── ArchiveBlockFetcher    ← later (B2 + sentinel fallback)
 └── coordinator: frontier (AtomicU64) + broadcast_tx   [single serialized point]
```

The division of labour:

- **`BlockStore`** persists and serves raw blocks. It is the source of
  truth for "where am I" (the resume point) and for gap detection. Concrete
  (one PG impl) — not a trait, since there is only one implementation.
- **`LiveSubscriber`** follows the chain tip. It is *always* a sentinel
  subscription regardless of backend, because B2 only ever serves cold
  history — the tip always comes from a node. Hence concrete, not a trait.
- **`BlockFetcher`** is the **only** axis that varies between the
  sentinel-only and the future B2-layered setup. So it is the **trait**, with
  the sentinel impl built now. It does **bounded** backfill: fetch a
  contiguous `[from, to]` range and terminate.
- **The coordinator** is the single serialized owner of `frontier` and the
  broadcast channel. Both writers (subscriber and fetcher) funnel through it
  so the `BlockSource` invariants (monotonic frontier, strict `+1` broadcast)
  hold by construction.

**Fetch is decoupled from store.** The fetcher knows nothing about Postgres;
the store knows nothing about sentinels or B2. The `RemoteBlockSource`
consumes from the fetcher and writes to the store. Swapping the fetcher
(sentinel → B2-layered) touches neither the store nor the projections.

## Lifecycle: transient backfill, then steady state

The defining property of this design: **the dual-writer phase is transient.**

- **Backfill (transient).** On a fresh or lagging start there are gaps below
  the live tip. Two writers run concurrently: the **subscriber** writes the
  live tail `[L..∞)`, the **fetcher** fills the gaps `[min..L-1]`. The store
  holds disjoint islands until the gaps close.
- **Steady state (single-writer).** Once every gap is filled the fetcher
  **dies**. Only the subscriber writes, each new block being exactly
  `frontier + 1`. The store is trivially contiguous and stays so.

This is why the subscriber captures `L` up front and the fetcher is
*bounded*: the backfill is a finite job that completes, collapsing the system
back to a single writer. (Contrast: a fetcher that chases a moving tip never
terminates, and you never leave the dual-writer regime.)

## Startup sequence

```
1. X = store.max_contiguous()              // highest H with [min..H] all present
   frontier = X                            // 0 / None on a fresh DB
2. L = subscriber.start()                  // current tip; spawn the task that
                                           // writes [L..) to the store + signals
3. gaps = store.gaps(min .. L)             // e.g. [(51,199),(211,249)]
4. for each gap, ascending:                // lowest-first — see Gap handling
       BlockFetcher.spawn(from, to)
       drain its FetchStream → store.put(h) + signal
5. coordinator advances `frontier` and broadcasts as the contiguous prefix grows
6. gaps exhausted → fetcher tasks dead → only the subscriber writes → steady state
```

The resume point comes from the **store**, never from the fetcher: the
fetcher is stateless about progress. This makes restarts trivial — recompute
`X`, recompute the gaps, refill.

## The store: `blocks_raw`

```sql
blocks_raw (height BIGINT PRIMARY KEY, payload BYTEA NOT NULL)
```

- **`put(h, data)` is idempotent** (`INSERT … ON CONFLICT (height) DO
  NOTHING`). Two writers may race at a boundary; the upsert makes overlap
  harmless.
- **`get(h)`** is a PK point query (sub-ms), used by projections in catch-up.
- **`max_contiguous()`** returns the highest `H` such that `[min..H]` are all
  present — seeds the frontier at boot.
- **`gaps(from, to)`** returns the maximal missing ranges in `[from, to)` via
  a window-function query (`LEAD`/`LAG` over `height`). This is where the
  "what's missing" intelligence lives — *not* in the fetcher.

`payload` is the borsh-serialized `BlockData { block, outcome }`. A
`u16 schema_version` carrier is an open question (see below) — cheap
insurance against future layout changes on a 22M+ row table.

## The live subscriber

Reuses the sentinel's `block` notification stream — the same GraphQL
subscription `LocalBlockSource` speaks, pointed at a remote sentinel rather
than an in-process `dango-httpd`. Open it, take the first delivered height as
`L`, then for each notified height **fetch the full `BlockData` via the same
sentinel RPC the fetcher uses** (`query_block` + `query_block_outcome`), write
it to the store, and signal the coordinator.

This is the key V1→V2 shift on the subscriber side: the notification carries
only the height (not the payload), and there is **no local disk to read the
payload from** as in V1 — so the subscriber fetches over RPC. The chain
produces blocks in order and the subscription delivers them in order, so
`[L..∞)` from the subscriber is contiguous.

Crucially the subscriber **writes immediately**, from `L` onward, *during*
the backfill. This is what bounds the fetcher (fixes its target at `L-1`) and
lets it die. If the subscriber did not write during backfill, by the time the
fetcher reached `L` the tip would have moved and the fetcher would chase it
forever.

What the subscriber does **not** do: it never broadcasts. See
[Write vs. broadcast](#write-vs-broadcast-the-key-distinction).

## The block fetcher

The fetcher does **bounded** backfill of one contiguous range and then
terminates. Borrowed almost verbatim from the bots `BlockFetcher`
(`bots/types/src/block_fetcher.rs`) — see [Borrowed pattern](#borrowed-pattern-bots-blockfetcher).

```rust
/// Pulls raw blocks from some backend (a sentinel node now; B2 + sentinel
/// later) and streams them, in strictly ascending height order, to the
/// `RemoteBlockSource` that owns it. Knows nothing about storage.
pub trait BlockFetcher: Send + Sync {
    /// Spawn the fetch task for the inclusive range `[from, to]`. The returned
    /// stream yields blocks ascending and terminates after `to`. Dropping the
    /// stream aborts the task. Sync — the work happens in the spawned task.
    fn spawn(&self, from: u64, to: u64) -> FetchStream;
}

/// Shared, backend-agnostic handle to a running fetcher: a bounded,
/// backpressured channel plus abort-on-drop. Every `BlockFetcher` impl feeds
/// this same type, so the consumer side is identical regardless of backend.
pub struct FetchStream {
    _abort: AbortOnDrop<()>,
    rx: mpsc::Receiver<BlockData>,
}

impl FetchStream {
    pub async fn recv(&mut self) -> Option<BlockData> { self.rx.recv().await }
    /// Fetched-but-not-yet-stored backlog — the reindex-bottleneck signal.
    pub fn queue_len(&self) -> usize { self.rx.len() }
}
```

Design notes:

- **Bounded, one range, dies.** `spawn(from, to)` is the primitive. The
  `RemoteBlockSource` invokes it **once per gap** (ascending). The fresh-start
  case is just the single gap `[min, L-1]`.
- **Per-gap, not gap-list.** Keeping the trait to a single range (rather than
  a list of ranges) keeps it minimal *and* composes with the future B2
  layered fetcher: each gap can be routed to the backend that owns that height
  range (old → B2, recent → sentinel) without the fetcher knowing anything.
- **Shared `FetchStream`.** The channel + backpressure + abort-on-drop are a
  concrete shared type so the store side is identical for every backend, and
  backpressure is uniform. The fetch *loop* (sentinel poll vs. B2 chunk read)
  is what each impl owns — this is option **A** from the design discussion
  (vary the whole loop, share only the handle), chosen because RPC batching
  and chunk reads are genuinely different acquisition strategies and forcing
  them behind a common `acquire(from,count)` would abstract on the wrong axis.

### `SentinelBlockFetcher` (built now)

Adapted from the bots `BlockFetcher`: a background task that pulls blocks from
a sentinel over RPC in **fixed-size batches** (fetch `batch_size` heights
concurrently, clamped to the blocks left in the gap), sends them through the
bounded channel in ascending order, and stops after `to`. No adaptive ramp:
the bots version grows/shrinks the batch because it also follows the chain
tip, but every height in a gap is below the live tip and therefore exists, so
a plain fixed batch suffices.

The one substantive difference from the bots version: bots throws away
everything but `block.info`, because points only needs the header. We keep the
**full `Block`** — `query_block` already returns it — and emit
`BlockData { block, outcome }`, since projections need txs and events.

## The coordinator: frontier + broadcast

Both writers funnel block-written signals to a **single serialized point**
(a mutex-guarded routine or a dedicated task) that owns `frontier` and the
broadcast sender. It is the only thing that mutates the frontier or sends on
the broadcast, which is what makes the `BlockSource` invariants hold trivially.

On each signal it advances the contiguous prefix: while `store` contains
`frontier + 1`, bump `frontier` and broadcast that block. In the common case
the block was just written and is in hand (no read-back); the store is read
only to **cross islands** left by a previous run (e.g. jumping `199 → 210`
over a pre-existing `200..210`).

```rust
// RemoteBlockSource::run, in spirit
let x = self.store.max_contiguous().await?;          // resume from own store
self.frontier.store(x, Ordering::Release);
let l = self.subscriber.start().await?;              // tip; subscriber writes [l..)
for (from, to) in self.store.gaps(min, l).await? {   // ascending, lowest-first
    let mut s = self.fetcher.spawn(from, to);
    while let Some(data) = s.recv().await {
        self.ingest(data).await?;                    // put → advance frontier → broadcast
    }
}
// fetchers done; only the subscriber feeds `ingest` from here on.

// ingest is the serialized point:
async fn ingest(&self, data: BlockData) -> AnyResult<()> {
    let h = data.height();
    self.store.put(h, &data).await?;                 // 1. persist (durable)
    // advance the contiguous prefix, broadcasting each newly-contiguous block
    self.advance_and_broadcast().await               // 2. frontier  3. broadcast
}
```

### Write vs. broadcast (the key distinction)

These are two different events and conflating them is the easy mistake:

- **Write to the store** happens immediately, by whichever writer produced
  the block. The subscriber writes `[L..∞)` even while the frontier is far
  below `L`.
- **Broadcast** happens only when the **frontier reaches** that block —
  because the broadcast must be strictly `+1` contiguous. A block `h` is
  physically un-broadcastable until `[min..h]` are all stored. The subscriber
  therefore *cannot* broadcast directly: it would emit `250` while the
  frontier is `50`, breaking the invariant and the projection loop.

Consequences:

- During backfill the pubsub is **not idle** — the coordinator broadcasts the
  *backfill* blocks as the frontier climbs (`51, 52, …, 210, 211, …`). There
  is no "switch on the live feed" moment; the live blocks (`≥ L`) are simply
  the next blocks in the contiguous sequence once the frontier arrives.
- A projection can **ride the broadcast during backfill**: as soon as it
  reaches the (still-climbing) frontier, `get()` returns `None`, it subscribes
  lazily, and consumes the broadcast at the rate the fetcher fills. It waits
  for the frontier to reach *it*, not for the whole backfill to finish.
- The instant the last gap closes, the frontier rips through the live blocks
  the subscriber had been accumulating in the store (a short catch-up burst),
  then settles into steady state.
- In **steady state** every new subscriber block already *is* `frontier + 1`,
  so it is broadcast immediately — full push latency.

## Gap handling

After a restart the store holds disjoint islands, so the backfill is a
**multi-gap** problem, not a single range. Worked example:

```
store on restart : {1..50, 200..210}
subscribe        : L = 250            (211..249 were produced during downtime)
gaps(min, 250)   : [(51,199), (211,249)]
```

- **The store computes the gaps**, in one window-function query. The fetcher
  stays dumb (fetch `[from,to]`); the "skip the islands" is implicit — those
  ranges simply aren't in the gap list.
- **Fill lowest-first.** If `[211..249]` were filled before `[51..199]`, the
  frontier would stay pinned at `50` (block `51` still missing) and **no
  projection could advance**. Ascending → the frontier climbs immediately and
  projections drain progressively. This is a liveness requirement, not a
  preference.
- **Clean cut.** The subscriber owns `[L..∞)`, the fetcher owns `[min..L-1]`,
  where `L` is the first height the subscription delivers. Using `L` as the
  gap query's upper bound is what captures the **downtime hole** (`211..249`)
  — blocks the subscriber will *not* backfill because it starts at the tip.

Rejected alternative: a single-range backfill from `max_contiguous` (`51..249`)
relying on the idempotent upsert to absorb the island. It works, but if the
island is large (the subscriber ran for hours in a previous session), it
re-fetches tens of thousands of already-present blocks. The gap query avoids
that for the cost of one query.

## Upholding the `BlockSource` invariants

The [four invariants the projection loop relies on](../DESIGN.md#blocksource-invariants-the-loop-relies-on),
and how this impl guarantees each:

| Invariant | How it holds |
|---|---|
| `contiguous_frontier()` monotonic | Only the coordinator mutates it, and only ever upward (advance the contiguous prefix). Jumps over islands are fine — monotonic ≠ `+1`. |
| `h ≤ frontier ⟹ get(h) = Some` | The frontier advances only after the block is durably in the store; the prefix it covers is contiguous by construction. |
| broadcast strictly `+1` | The coordinator is the single broadcaster and sends a block only at the moment it advances the frontier onto it. |
| `get(h) = None` is "not yet" | A height inside an unfilled gap returns `None`; the source never GCs a height a projection might still reach (no GC in this version at all). |

The fixed ordering inside `ingest` — **store commit → frontier → broadcast** —
is what ties the first three together. Reversing it (broadcast before the
store commit) would let a projection `get()` a height that isn't durable yet.

## Failure modes

| Failure | Behavior |
|---|---|
| Subscriber connection drops | Reconnect with backoff; resume from the tip. Any blocks missed during the outage become a gap below the new `L` on the next gap recomputation, filled by a fetcher. Frontier stalls at the contiguous prefix meanwhile. |
| Sentinel down (no node reachable) | Both subscriber and fetcher retry. Frontier stalls; projections keep serving up to the old frontier. No data loss — the store is durable. |
| Fetcher RPC error / 404 mid-gap | Adaptive loop drops to single-block, sleeps, retries (bots pattern). A 404 inside a gap means "not yet on the sentinel" — retry, don't treat as absent. |
| Crash mid-backfill | Restart recomputes `X` and the gaps from the store; refills. Idempotent `put` makes any partially-written block harmless. |
| Crash between store `put` and frontier advance | On restart the block is in the store, `max_contiguous` picks it up, frontier seeds correctly. No hole. |
| Corrupt payload in `blocks_raw` | `get` surfaces a borsh error via `anyhow`. Skip-vs-halt policy is an open question (probably halt — unlike V1 there is no in-process indexer cross-checking). |

## Borrowed pattern: bots `BlockFetcher`

`bots/types/src/block_fetcher.rs` already solves the hard part and we lift it:

- **background task + `AbortOnDrop` + channel + `recv()`** — the "consume
  blocks in order" interface, ready-made.
- **concurrent batches** — fetch `batch_size` heights at once instead of one
  by one; essential for genesis backfill (22M sequential blocks would be
  unwatchable). We drop the bots' adaptive ramp: it exists to follow the tip,
  which a bounded gap never reaches.
- **ordered output despite parallel fetch** — sends `block, block+1, …` in
  sequence and `break`s on the first error to retry from there, so the stream
  stays contiguous (which the store and frontier require).
- **bounded channel** (the `bots/types` copy is capacity 10k → backpressure;
  the `dango/types` copy is `unbounded` — **use the bounded one**, the
  unbounded balloons RAM during a reindex).

What we change: emit the **full `Block`** (not `block.info`), and add the
`from/to` bound so the task terminates after the gap (bots runs open-ended
with an optional `max_block`).

## Known ceilings & trade-offs (accepted for this version)

- **2 RPC/block × 22M ≈ 44M calls** for a genesis backfill. Acceptable now —
  it reuses proven code and the sentinel currently holds everything. The
  future optimization is a streaming RPC (`SubscribeBlocks`-style, ~1000+
  blk/s replay) when the sentinel exposes one; the bounded-fetcher trait can
  wrap it without changing the source.
- **`blocks_raw` grows unbounded.** ~210 GB today, ~600 GB/yr, ~2 TB at 5
  years (zstd-dict; uncompressed borsh in PG will be larger). GC is deferred —
  see below.
- **Live latency.** The subscriber gives push latency at the tip (sub-second),
  better than a poll loop, which is the whole reason for keeping a separate
  subscriber rather than letting the fetcher poll the tip.

## Configuration

Source-specific options under a `remote.*` section, as promised by
[DESIGN.md](../DESIGN.md#configuration) (the top-level `source = remote`,
`pubsub_buffer_size`, and `postgres.*` live there):

| Field | Description |
|---|---|
| `remote.sentinel_url` | Base URL of the sentinel — both its `block` subscription (live tail) and its block RPC (`query_block` / `query_block_outcome`, used by the subscriber and the fetcher). |
| `remote.fetch_batch_size` | Blocks fetched concurrently per backfill batch (clamped to the blocks left in a gap). Bounds load on the sentinel. |
| `remote.fetch_timeout` | Per-batch RPC timeout before the batch is retried. |

The raw-block store (`blocks_raw`) lives in the **same indexer-owned Postgres**
as `projection_cursors` and the PG projections (`postgres.*`). The source
writes it **independently** — it does *not* participate in the projection
[commit protocol](../DESIGN.md#commit-protocol) transaction, so co-locating it
in that database is a convenience, not a correctness requirement.

## End goal: B2-layered fetcher

*Not built now; recorded so this version's decisions don't close the door.*

The end-state replaces `SentinelBlockFetcher` with an `ArchiveBlockFetcher`
that pulls cold history from B2 and recent blocks from a sentinel — same
`BlockFetcher` trait, same `spawn(from, to)`. Because the source already
drives the fetcher **per gap**, the layered fetcher can route each gap by
height: old ranges → B2 chunks, recent ranges → sentinel hot window. Nothing
in the store, coordinator, subscriber, or projections changes.

The B2 side (chunk format, zstd dictionary, `HEAD`/manifests, overlap
invariant between the sentinel hot window and the chunk cadence) is specified
in `~/specs/left-curve/indexer-storage-architecture.md`.

**Storage cost** (raw zstd-dict, for the GC discussion):

| Period | Raw zstd-dict |
|---|---|
| Today (~22M blocks) | ~210 GB |
| 1 year | ~600 GB |
| 5 years | ~2 TB |

**Optional GC** (when storage becomes a concern): when all registered
projections are past height `H`, drop blocks below `H`. New projections
deployed after a GC fetch the missing blocks transparently via the fetcher →
B2. Implementation detail of the source; no change to the public trait. This
is the one place the `get(h) = None` invariant interacts with GC: a GC'd
height must be re-fetchable, never reported as permanently absent.

## Open questions

- **Subscriber wire protocol** — WS / SSE / gRPC against the sentinel. Reuse
  the GraphQL subscription `LocalBlockSource` already speaks, or a dedicated
  block stream? Trade-offs on backpressure and reconnection.
- **Fetcher error model** — structured errors (e.g.
  `NotAvailable { earliest: u64 }`, needed once B2 lands to signal "older than
  the archive") vs. plain 404s for the sentinel-only version.
- **`blocks_raw` partitioning** — partition by height range (e.g. every 10M)
  up front, so a future GC is a partition drop rather than a row delete?
- **Schema version on the payload** — include a `u16 schema_version` from day
  one to avoid painful migrations on a 22M+ row table.
- **Corrupt-payload policy** — skip + alert vs. halt. Leaning halt (no
  in-process indexer to cross-check here), but wants a config knob.
- **Parallel gap filling** — fill multiple gaps concurrently for throughput?
  Deferred; sequential lowest-first is simpler and the projection processing,
  not the fetch, is usually the bottleneck.
