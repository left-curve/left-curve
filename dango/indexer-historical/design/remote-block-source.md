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
 ├── BlockStore (PG blocks_raw)              [trait, PG impl + memory for tests]
 │     put · get · max_contiguous · max_height · gaps — idempotent put
 ├── LiveSubscriber (subscription to a sentinel) [trait, one real impl]
 │     yields L at startup, then yields the live tail [L..∞) as a stream
 ├── Arc<dyn BlockFetcher>                       [trait, ≥1 impl]
 │     spawn(from, to) -> FetchStream   (bounded backfill, dies at `to`)
 │       ├── SentinelBlockFetcher   ← built now
 │       └── ArchiveBlockFetcher    ← later (B2 + sentinel fallback)
 └── coordinator: frontier (AtomicU64) + broadcast_tx   [single serialized point]
```

The division of labour:

- **`BlockStore`** persists and serves raw blocks. It is the source of
  truth for "where am I" (the resume point) and for gap detection. A trait
  (the PG impl in production, an in-memory one for tests), though there is only
  one production backend.
- **`LiveSubscriber`** follows the chain tip. It is *always* a sentinel
  subscription regardless of backend, because B2 only ever serves cold history
  — the tip always comes from a node. A trait too (one real impl, mocked in
  tests). It is a pure producer: it *yields* blocks; the coordinator is the
  single writer to the store.
- **`BlockFetcher`** is the **only** axis that varies between the
  sentinel-only and the future B2-layered setup. So it is the **trait**, with
  the sentinel impl built now. It does **bounded** backfill: fetch a
  contiguous `[from, to]` range and terminate.
- **The coordinator** is the single serialized owner of `frontier` and the
  broadcast channel. Both writers (subscriber and fetcher) funnel through it
  so the `BlockSource` invariants (monotonic frontier, ascending broadcast)
  hold by construction. It tracks the stored-but-not-yet-contiguous blocks
  (**islands**) in memory and jumps the frontier across them.

**Fetch is decoupled from store.** The fetcher knows nothing about Postgres;
the store knows nothing about sentinels or B2. The `RemoteBlockSource`
consumes from the fetcher and writes to the store. Swapping the fetcher
(sentinel → B2-layered) touches neither the store nor the projections.

## Lifecycle: backfill, then steady state — healed continuously

- **Backfill.** On a fresh or lagging start there are gaps below the live tip.
  Two writers run concurrently: the **subscriber** writes the live tail
  `[L..∞)`, and a **healer** fills the gaps `[min..L-1]`, spawning a *bounded*
  fetcher per gap. The store holds disjoint islands until the gaps close.
- **Steady state.** Once every gap is filled the healer goes idle and only the
  subscriber writes, each new block being exactly `frontier + 1`. The store is
  trivially contiguous and stays so.
- **Re-healing.** The healer never exits — it keeps watching. If the subscriber
  reconnects at a higher tip (a downtime hole) or drops a block, that hole is
  just a new gap below the tip; the healer notices (a discontinuity signal, or
  a periodic re-check) and re-fills it. The system briefly re-enters the
  dual-writer regime and settles back.

Each fetcher is still **bounded** (one gap, then it dies) — that is what keeps
backfill from chasing a moving tip. The *healer* is the continuous supervisor
that spawns one bounded fetcher per gap whenever a gap exists. The subscriber
captures the tip up front and writes the live tail during backfill, which is
what fixes each fetcher's target below the tip.

## Startup sequence

```
1. X = store.max_contiguous()   // resume point; frontier = X (0 on a fresh DB)
2. spawn coordinator            // serialized frontier + broadcast (see below)
3. spawn drain_live             // owns the subscription: (re)subscribe, feed
                                //   blocks to the coordinator, track the tip
4. spawn healer                 // loop: gaps(frontier+1, tip) → fill the lowest
                                //   via a bounded fetcher; idle when none
```

All three run for the source's lifetime; whichever returns first tears the
others down. The resume point comes from the **store**, never from the fetcher
(stateless about progress), so a restart just recomputes `X` and the gaps and
refills. There is no one-shot "backfill then stop" phase — the healer keeps
`[min..tip)` gap-free, whether the gap is the initial history or a later hole.

## The store: `blocks_raw`

```sql
blocks_raw (height BIGINT PRIMARY KEY, payload BYTEA NOT NULL)
```

- **`put(h, data)` is idempotent** (`INSERT … ON CONFLICT (height) DO
  NOTHING`). The coordinator is the single writer, so the upsert is belt-and-
  suspenders — it absorbs a re-fetched block on restart or a boundary overlap.
- **`get(h)`** is a PK point query (sub-ms), used by projections in catch-up.
- **`max_contiguous()`** returns the highest `H` such that `[min..H]` are all
  present — seeds the frontier at boot.
- **`max_height()`** returns the highest stored height — bounds the in-memory
  island scan at boot.
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
sentinel RPC the fetcher uses** (`query_block` + `query_block_outcome`) and
yield it. It is a pure producer: `drain_live` forwards the yielded blocks to the
coordinator (the single store writer) and tracks the tip; reconnection lives in
`drain_live`, so the concrete subscriber only needs to open one subscription and
yield.

This is the key V1→V2 shift on the subscriber side: the notification carries
only the height (not the payload), and there is **no local disk to read the
payload from** as in V1 — so the subscriber fetches over RPC. The chain
produces blocks in order and the subscription delivers them in order, so
`[L..∞)` from the subscriber is contiguous.

Crucially the live tail is **stored immediately**, from `L` onward, *during*
the backfill (the coordinator persists each yielded block). This is what bounds
the fetcher (fixes its target at `L-1`) and lets it die: if the live tail were
not stored during backfill, by the time the fetcher reached `L` the tip would
have moved and the fetcher would chase it forever.

What the subscriber does **not** do: it never writes the store or broadcasts —
both are the coordinator's job. See
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

The coordinator is a single task draining a bounded channel fed by both writers
(the subscriber via `drain_live`, the healer via its fetcher). It is the only
thing that mutates `frontier` or sends on the broadcast. Per block it classifies
against the frontier:

- **edge** (`height == frontier + 1`) — extends the prefix: **broadcast →
  persist → advance**, then cross any contiguous islands;
- **island** (`height > frontier + 1`) — not contiguous yet: persist it and
  remember its height in an in-memory coalesced range set (`Islands`);
- **duplicate** (`height ≤ frontier`) — persist (idempotent) and ignore.

```rust
// RemoteBlockSource::run, in spirit
let x = self.store.max_contiguous(min).await?;        // resume from own store
self.frontier.store(x, Ordering::Release);
let (tx, rx) = mpsc::channel(coordinator_buffer);
let coordinator = spawn(self.run_coordinator(rx));    // the serialized point, below
let drain       = spawn(self.drain_live(tx.clone())); // subscription + reconnect loop
let healer      = spawn(self.run_healer(tx));         // continuous gap-filling
select_all([coordinator, drain, healer]).await;       // first to return tears down the rest

// run_coordinator, on an edge block (height == frontier + 1):
self.broadcast_tx.send(block.clone()).ok();           // 1. live projections, no store wait
self.store.put(h, &block).await?;                     // 2. persist (durable)
self.frontier.store(h, Ordering::Release);            // 3. advance — stays behind the store
while let Some(end) = islands.take_starting_at(frontier + 1) {  // cross islands in one step
    self.broadcast_tx.send(store.get(end)?).ok();     // bulk-advance: broadcast only the top
    self.frontier.store(end, Ordering::Release);
}
```

**Broadcast before persist (D1).** The broadcast goes out first, so live
projections index without waiting on the `blocks_raw` write; the frontier
advances *after* the block is durable, so `h ≤ frontier ⟹ get(h) = Some` still
holds (in the window between, the frontier is still `h-1`). A crash in that
window self-heals: the un-stored height is re-fetched as a gap on restart.

**Islands in memory, bulk-advance.** Islands are coalesced ranges held in
memory (seeded at boot from `store.gaps`), so an edge advances with **no store
probe**. Crossing a large island — a restart backlog, or the live tail
accumulated during backfill — jumps the frontier to its top and broadcasts only
that top, so the pubsub is not flooded; projections pull the skipped heights via
Phase-1 `get()`.

### Write vs. broadcast (the key distinction)

These are two different events and conflating them is the easy mistake:

- **Write to the store** happens immediately, by whichever writer produced
  the block. The subscriber writes `[L..∞)` even while the frontier is far
  below `L`.
- **Broadcast** happens only when the **frontier reaches** that block — the
  broadcast tracks the contiguous prefix. A block `h` is un-broadcastable until
  `[min..h]` are all stored. The subscriber therefore *cannot* broadcast
  directly: it would emit `250` while the frontier is `50`, breaking the
  projection loop.

Consequences:

- During backfill the pubsub is **not idle** — the coordinator broadcasts the
  *backfill* blocks as the frontier climbs (`51, 52, …, 210, 211, …`). There
  is no "switch on the live feed" moment; the live blocks (`≥ L`) are simply
  the next blocks in the contiguous sequence once the frontier arrives.
- A projection can **ride the broadcast during backfill**: as soon as it
  reaches the (still-climbing) frontier, `get()` returns `None`, it subscribes
  lazily, and consumes the broadcast at the rate the fetcher fills. It waits
  for the frontier to reach *it*, not for the whole backfill to finish.
- The instant the last gap closes, the frontier crosses the live-tail island
  the subscriber accumulated in the store in **one bulk step** — broadcasting
  only its top — and projections pull the rest via Phase-1 `get()`. Then it
  settles into steady state.
- In **steady state** every new subscriber block already *is* `frontier + 1`,
  so it is broadcast immediately — full push latency.

## Gap handling

The store can hold disjoint islands (a restart, or a later reconnect hole), so
gap-filling is a **multi-gap** problem the healer recomputes each pass, not a
one-shot single range. Worked example:

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
- **Tip as the upper bound.** The healer fills `gaps(frontier + 1, tip)`, where
  `tip` is the highest height the subscriber has delivered. Using the tip
  captures the **downtime hole** (`211..249`) — blocks the subscriber will not
  backfill because it resumes at the tip. The healer recomputes the gaps every
  pass, so a hole that appears later (a reconnect, a dropped block) is filled
  the same way.

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
| broadcast strictly ascending (may skip) | Only the coordinator broadcasts. Normally `+1`; crossing an island it jumps the frontier to the top and broadcasts only that, which the projection loop handles via Phase-1 pull. |
| `get(h) = None` is "not yet" | A height inside an unfilled gap returns `None`; the source never GCs a height a projection might still reach (no GC in this version at all). |

The ordering inside the coordinator — **broadcast → store commit → frontier**
(D1) — ties the first three together. The frontier advances last, only after
the block is durable, so it never claims a height `get()` cannot serve; the
broadcast running ahead of it is safe because no consumer validates a broadcast
block against `contiguous_frontier()` — the projection loop consumes it directly
and falls back to a `get()` pull on a forward jump.

## Failure modes

| Failure | Behavior |
|---|---|
| Subscriber connection drops | `drain_live` reconnects with backoff and resumes at the new tip. Blocks missed during the outage become a gap below the new tip; the healer detects it and fills it through a fetcher. Frontier stalls at the contiguous prefix meanwhile. |
| Subscriber drops a block mid-stream | A discontinuity in the delivered heights signals the healer, which fills the hole from the sentinel RPC (the block exists; only the live *notification* was lost). A periodic re-check is the backstop. |
| Sentinel down (no node reachable) | Both subscriber and fetcher retry. Frontier stalls; projections keep serving up to the old frontier. No data loss — the store is durable. |
| Fetcher RPC error / 404 mid-gap | The fetch task backs off and retries from the failed height (fixed-size batches, no adaptive ramp). A 404 inside a gap means "not yet on the sentinel" — retry, don't treat as absent. A *permanently*-unservable block would retry forever — tracked as a future observability item (known-issues #5). |
| Crash mid-backfill | Restart recomputes `X` and the gaps from the store; refills. Idempotent `put` makes any partially-written block harmless. |
| Crash between store `put` and frontier advance | On restart the block is in the store, `max_contiguous` picks it up, frontier seeds correctly. No hole. |
| Store `put` / `get` fails (transient PG error) | Coordinator halts, source exits — intentionally fatal. A process restart recovers: idempotent `put` plus the frontier re-seeding from `max_contiguous` mean no hole. In-place retry is deliberately avoided (the store is the durability anchor; halt + supervised restart beats limping on). |
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

The tunables exist today as Rust config structs with `Default`s; the CLI/TOML
wiring that maps a `remote.*` section onto them is not built yet.

`RemoteBlockSourceConfig` — `pubsub_buffer_size` (broadcast capacity),
`coordinator_buffer` (the coordinator's input channel), `heal_poll_interval`
(the healer's periodic re-check), `reorder_grace` (delay after a discontinuity
signal, to absorb an out-of-order delivery before fetching), `reconnect_backoff`
(between live re-subscribes).

`SentinelFetcherConfig` — `batch_size` (heights fetched concurrently per gap
batch), `timeout` (per-batch RPC timeout), `channel_capacity` (fetch-ahead
backlog — the dominant backfill-RAM knob), `retry_backoff`.

Still needed: `sentinel_url` (the sentinel's `block` subscription + its block
RPC), which lands with the concrete `LiveSubscriber` and its client.

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
