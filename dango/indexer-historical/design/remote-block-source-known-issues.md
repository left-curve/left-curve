# `RemoteBlockSource` — known issues & review findings

Review of the **in-progress** `RemoteBlockSource` (V2) and its collaborators,
captured so the gaps can be fixed incrementally. Each item has a severity, a
type, the code location, and a fix direction.

> See [remote-block-source.md](./remote-block-source.md) for the design and
> [DESIGN.md](../DESIGN.md) for the shared `BlockSource` contract. Scope of this
> review: the remote path of the `block-source` crate (`remote/mod.rs`,
> `remote/fetcher/`, `remote/subscriber.rs`, `remote/store/`). `LocalBlockSource`
> and the projection/committer side were reviewed and are clean.

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

## Severity index

| # | Issue | Severity | Type |
|---|---|---|---|
| 5 | Fetcher retries forever, silently, on a permanently-missing block | Medium | Robustness (observability) |

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
