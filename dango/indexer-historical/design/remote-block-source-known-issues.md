# `RemoteBlockSource` — known issues & review findings

Review of the **in-progress** `RemoteBlockSource` (V2) and its collaborators,
captured so the gaps can be fixed incrementally. Each item has a severity, a
type, the code location, and a fix direction.

> See [remote-block-source.md](./remote-block-source.md) for the design and
> [DESIGN.md](../DESIGN.md) for the shared `BlockSource` contract. Scope of this
> review: the remote path of the `block-source` crate (`remote/mod.rs`,
> `remote/fetcher/`, `remote/subscriber.rs`, `remote/store/`) **combined with the
> `RocksdbBlockStore` disk store**. `LocalBlockSource` and the
> projection/committer side were reviewed and are clean; the one cross-cutting
> genesis-floor interaction with the projection loop that this review surfaced
> has since been fixed (see **Fixed** below).

Date: 2026-06-23. Resolved items have been implemented and removed from this
tracker: the continuous healer, channel-capacity config, `select_all` teardown,
live-stream reconnect, and bulk-advance across islands. The architecture was
then refactored so the **store owns the topology** (frontier + gaps): `put`
reports the frontier advance, the coordinator is a thin broadcast driver with no
atomics, ordering is **persist → broadcast**, and the durable store is built
(`RocksdbBlockStore`, topology checkpointed atomically with each block). A
store-write error halting the source was reviewed and accepted as intended (see
the failure-modes table in [remote-block-source.md](./remote-block-source.md)).
See git history. What remains below is the open work.

**Updated 2026-06-23** with the scaling/runtime findings from the combined
`RemoteBlockSource` + `RocksdbBlockStore` review (#6–#15). The design's logic is
sound and the main desync paths are correct and unit-tested; these items are
behaviors that surface only **at 100M+ blocks, with large payloads, or under
concurrent projection load** — exactly the regime the store is meant for. None
is a logic bug in the steady state; #6 is the one that defeats an explicit design
promise.

**Fixed 2026-06-23 (same review).** Five of those landed with tests and are
removed from the tracker below, summarized here (see git history for the diffs):

- **#6 — topology rebuild read every blob.** `rebuild_topology` now walks a
  `raw_iterator_cf` reading only the 8-byte keys (never `value()`), so a rebuild
  no longer resolves blobs. Tests: `rebuilds_topology_when_checkpoint_absent`,
  `rebuilds_topology_when_checkpoint_is_corrupt`.
- **#7 — blocking RocksDB I/O on the async workers.** `RocksdbBlockStore` now
  holds `Arc<DB>`; `get` (read + decode) and `put` (the write) run on
  `tokio::task::spawn_blocking`, off the async workers. The block encode stays on
  the caller (the coordinator is the only writer, so at most one is in flight).
- **#8 — one bad block took the whole source down.** The healer now catches a
  `backfill_gap` failure, logs it, backs off (`heal_retry_backoff`, default 1 s),
  and retries the gap instead of propagating — non-fatal to the source. Genuine
  store-write errors stay fatal (they surface from the coordinator).
- **#11 — projection livelock below the genesis floor.** `projection_loop` clamps
  the cursor to `GENESIS_HEIGHT`, now exported from the block-source crate.
- **#12 — corrupt checkpoint failed boot.** `open` falls back to the key-only
  rebuild on a checkpoint decode error instead of returning `Err`.

**Fixed 2026-06-24 — measured against mainnet.** The two scaling items landed,
tuned against live data instead of the earlier estimates. Mainnet on 2026-06-24:
**~32.5M blocks** growing at **~2.79 blk/s** (→ ~100M in ~9 months); `BlockData`
payloads **median ~15-25 KB, p90 ~150 KB, max ~0.5 MB borsh** — far above the
design's 9.5 KB/block historical average (the chain is busier now).

- **#10 — blocks-CF tuning.** `blocks_cf_options`: block cache 256 MB → **1 GiB**
  (index+filter alone are ~350-450 MB at 100M keys), **partitioned** index+filter
  (`TwoLevelIndexSearch` + `partition_filters` + `pin_top_level_index_and_filter`,
  `format_version 5`) so data blocks no longer evict them, a dedicated **512 MiB
  blob cache** (payloads were uncached), and `max_background_jobs=6` /
  `max_subcompactions=4` for the backfill burst. `min_blob_size` stays 4 KB —
  the measured payloads are well above it, so ~all go to blobs.
- **#9 — buffer RAM.** `pubsub_buffer_size` and the sentinel `channel_capacity`
  10_000 → **2_000** (≈ 40 MB typical / 300 MB peak each at the measured payloads,
  vs multi-GB before); the doc-comments now size in bytes. Lagged projections are
  caught by the Phase-1 `get()` recovery, so the smaller ring costs nothing.

## Severity index

| # | Issue | Severity | Type |
|---|---|---|---|
| 5 | Fetcher retries forever, silently, on a permanently-missing block | Medium | Robustness (observability) |
| 13 | First-write-wins: a re-delivered different payload for a stored height is ignored | Low | Correctness (assumption) |
| 14 | Topology checkpoint rewritten on every `put` (write amplification) | Low | Scaling (efficiency) |
| 15 | `reorder_grace` applied only after a notify, not after the periodic poll | Low | Cosmetic |

## Completeness — what is not built yet

Not bugs, but the source cannot run in production until these land:

- **No concrete `LiveSubscriber`.** Only the trait. The sentinel subscriber
  (height notification → `query_block` / `query_block_outcome` → yield) is
  unwritten. `drain_live` already owns the reconnect loop, so the concrete impl
  only needs to open one subscription and yield blocks.
- **No wiring.** `cli/src/main.rs` is a stub; nothing maps a `remote.*` config
  section onto `RemoteBlockSourceConfig` / `SentinelFetcherConfig` or the store
  path yet.
- **Tests:** the coordinator + healer + reconnect + bulk-advance paths and the
  RocksDB store (put/get, topology, idempotency, reopen-from-checkpoint) are
  covered by unit tests using `MemoryBlockStore` / a temp RocksDB + mock
  subscriber/fetcher. Still untested: the (unbuilt) sentinel subscriber.

---

## 5. Fetcher retries forever, silently, on a permanently-missing block — Medium

**Location:** `remote/fetcher/sentinel.rs` — `fetch_range` (timeout + error
backoff paths).

On any RPC error/timeout the fetcher backs off and retries **forever** (the
assumption "every block in a gap exists below the tip"). True today, but if a
block is genuinely unservable (pruning, a mis-computed gap), the gap never
closes → `backfill_gap` never returns → the **healer** is stuck on that gap →
frozen frontier, again **silently** (the healer re-detects *new* holes each
pass, but cannot get past one it can never fill). No attempt limit, no
escalation.

**Mitigation (deferred — observability, not code).** There is no safe code
workaround: every height in a gap is supposed to exist, so the fetcher *must*
keep retrying — treating a transient 404 as "absent" would punch a permanent
hole. The plan is to **detect** the condition instead: a future metric such as a
"backfill made no progress for N seconds" / "healer stuck on gap" gauge that
alerts an operator, rather than changing the retry policy. Tracked here so the
metric is not forgotten when observability lands.

---

## 13. First-write-wins: a re-delivered different payload is ignored — Low

**Location:** `remote/store/disk.rs` and `remote/store/memory.rs` — the
idempotent `contains(height) → return None` guard in `put`.

`put` is keyed by height only: once a height is stored, a later `put` of the same
height — even with a *different* payload — is a no-op. For finalized CometBFT
blocks (no reorgs past finality) this is correct and desirable. But it is an
**unstated assumption**: if a reconnecting subscriber or a pre-finality reorg
ever delivered a different block for a stored height, the store would silently
keep the first one.

**Fix direction.** Make the finality assumption explicit in the `BlockStore`
docs; optionally carry/compare a block hash on re-put and warn on a mismatch
(ties into the schema-version / integrity-hash carrier-metadata idea in
`DESIGN.md`).

---

## 14. Topology checkpoint rewritten on every `put` — Low

**Location:** `remote/store/disk.rs` — `put` (the `batch.put(TOPOLOGY_KEY, ..)`).

Each `put` rewrites the checkpoint key in the default CF, so a 100M-block
backfill is 100M overwrites of one key — extra write amplification on the default
CF during the burst (it compacts down to one live key, but the churn is real).
The current shape is the price of crash-consistency (checkpoint atomic with the
block), which is the right default.

**Fix direction.** If it shows up in practice, checkpoint periodically (every N
blocks or on a timer) and rebuild only the uncheckpointed tail at boot — the
key-only scan from #6 makes a bounded tail-rebuild cheap. Not worth doing
pre-emptively.

---

## 15. `reorder_grace` applied only after a notify, not after the poll — Low

**Location:** `remote/mod.rs` — `run_healer` (the `tokio::select!` idle arm).

The grace delay that lets an out-of-order live delivery land runs only on the
`heal_notify` branch; a wake from the periodic `heal_poll_interval` re-checks
`lowest_gap` immediately. So a poll that coincides with a block still in flight
can spawn a redundant fetch. It is harmless — `put` is idempotent, whichever
writer lands first wins and the other is a no-op — purely a wasted request.

**Fix direction.** Optional: apply the same short grace on the poll arm, or leave
as-is given it is idempotent. Cosmetic.
