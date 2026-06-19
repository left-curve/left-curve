# `RemoteBlockSource` — known issues & review findings

Review of the **in-progress** `RemoteBlockSource` (V2) and its collaborators,
captured so the gaps can be fixed incrementally. Each item has a severity, a
type, the code location, and a fix direction.

> See [remote-block-source.md](./remote-block-source.md) for the design and
> [DESIGN.md](../DESIGN.md) for the shared `BlockSource` contract. Scope of this
> review: the remote path of the `block-source` crate
> (`block_source/remote.rs`, `block_fetcher/`, `live_subscriber/`,
> `block_store/`). `LocalBlockSource` and the projection/committer side were
> reviewed and are clean.

Date: 2026-06-19. Resolved items (the continuous healer, channel-capacity
config, the `select_all` task teardown, and broadcast-before-store) have been
implemented and removed from this tracker — see git history. What remains below
is the open work.

## Severity index

| # | Issue | Severity | Type |
|---|---|---|---|
| 2 | No live-stream resilience — one stream error kills the source (and the app) | **High** | Robustness |
| 5 | Fetcher retries forever, silently, on a permanently-missing block | Medium | Robustness |
| 6 | Store errors are not retried — a transient PG blip kills everything | Medium | Robustness |
| 7 | One wasted `store.get()` per block for the whole backfill | Low–Medium | Efficiency |

## Completeness — what is not built yet

Not bugs, but the source cannot run in production until these land:

- **No Postgres `BlockStore`** (`blocks_raw`). Only `MemoryBlockStore` exists.
  Production needs the durable store with `max_contiguous` / `gaps` as window
  queries (`LEAD`/`LAG` over `height`).
- **No concrete `LiveSubscriber`.** Only the trait. The sentinel subscriber
  (height notification → `query_block` / `query_block_outcome` → store → signal)
  is unwritten.
- **No wiring.** `cli/src/main.rs` is `fn main() {}`; `RemoteBlockSource` is
  referenced only inside its own crate. The tunables exist as
  `RemoteBlockSourceConfig` / `SentinelFetcherConfig` defaults, but nothing maps
  a `remote.*` config section onto them yet.
- **Tests:** the coordinator + healer are covered by unit tests in `remote.rs`
  (in-order, reorder, island-crossing, skip-heal, reconnect-heal,
  broadcast-before-store) using `MemoryBlockStore` + mock subscriber/fetcher.
  Still untested: the (unbuilt) Postgres store and sentinel subscriber.

---

## 2. No live-stream resilience — **High**

**Location:** `block_source/remote.rs` — `drain_live` and `run`'s `select_all`
teardown.

**Symptom.** When the live stream ends (`None`) or yields an error, `drain_live`
returns; `run`'s `select_all` then tears the other tasks down and `run` returns
→ `App::run` completes → **the whole indexer shuts down.** A single transient
subscription error takes the process down; there is no reconnect.

The design assigns reconnection to the subscriber ("the subscriber owns
reconnection"), but there is no concrete subscriber yet, so today nothing
reconnects.

**Fix direction.** Either (a) the concrete `LiveSubscriber` owns reconnect +
contiguity (preferred — keeps `RemoteBlockSource` simple), or (b) `run` wraps
the subscribe + drain in a retry loop with backoff. With the healer in place,
the downtime hole a reconnect leaves is repaired automatically — so what remains
here is purely **keeping the subscription alive** (reconnect with backoff)
instead of letting a stream end/error tear the source down. Decide where the
responsibility sits before writing the sentinel subscriber.

---

## 5. Fetcher retries forever, silently, on a permanently-missing block — Medium

**Location:** `block_fetcher/sentinel.rs` — `fetch_range` (timeout + error
backoff paths).

On any RPC error/timeout the fetcher backs off and retries **forever** (the
assumption "every block in a gap exists below the tip"). True today, but if a
block is genuinely unservable (pruning, a mis-computed gap), the gap never
closes → `backfill_gap` never returns → the **healer** is stuck on that gap →
frozen frontier, again **silently** (the healer re-detects *new* holes each
pass, but cannot get past one it can never fill). No attempt limit, no
escalation.

**Fix direction.** A bounded retry / escalation after N attempts (surface a
structured error up to the source), and/or the structured fetcher error model
already noted as an open question in the design (`NotAvailable { earliest }`).

---

## 6. Store errors are not retried — Medium

**Location:** `block_source/remote.rs` — `run_coordinator` `store.put` / sweep
`store.get` (`?`-propagated).

Any error from `store.put` / `store.get` bails the coordinator → kills the
source → kills the app. A transient PG blip takes everything down.

**Fix direction.** Retry store operations with backoff inside the coordinator
(the store is the durability anchor; a transient failure should not be fatal).

---

## 7. One wasted `store.get()` per block for the whole backfill — Low–Medium

**Location:** `block_source/remote.rs` — `run_coordinator` sweep loop.

During an in-order backfill every block is `frontier + 1`, so after advancing,
the sweep does `store.get(height + 1)` which returns `None` (the next block is
not in yet). That is **one wasted PG round-trip per block** — ~22M extra
queries over a genesis backfill. Correct, but costly.

**Fix direction.** Only attempt the cross-island sweep when there is reason to
(e.g. track the known islands from `gaps()` and sweep only when the just-filled
height reaches an island boundary), instead of probing the store after every
single block.
