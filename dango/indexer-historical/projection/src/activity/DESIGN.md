# Activity projection — design

The foundational projection of the historical indexer. For every block it
produces three Postgres tables that together answer, for any account address,
**"what did it do and what touched it"**:

- `transactions` — one row per executed unit (a transaction *or* a cronjob);
- `events` — the **merged event log + participation index**: one row per
  *(event × participant address)*, plus an empty-address row for kept events
  with no participant and a sentinel row per tx sender;
- `event_data` — the event payload (`zstd(borsh(event))`), split out and kept
  only for the configured priority types.

It is a **single projection**: one `process(block)` flattens the block's
events once and writes all three tables in one commit (one `Ctx`, one cursor).

## Queries it must serve

All paginated **newest-first** (`block_height DESC`, then in-block position),
keyset-paginated (cursor on the ordering tuple, never `OFFSET`):

1. transactions involving an address X (X is the sender, or a party in one of
   the unit's events);
2. transactions where X is specifically the sender;
3. events filtered by **event type** (e.g. `transfer`, `contract_event`);
4. contract events filtered by **name** (e.g. `order_filled`, `liquidated`);
5. events where an address X **participates**;
6. events emitted **by a contract** C;
7. and the combinations of 3/4/6 **with** an involved address X — e.g. "the
   `order_filled` events involving X", "the gateway's events involving X".

## Why one merged `events` table (not events + involvement)

An earlier design had two tables: a narrow `events` (one row per event) and an
`involvement` fan-out (one row per participant) carrying the event's filter
attributes *denormalized* so address+attribute queries (query 7) stay a single
seek. But once the attributes are denormalized, the two tables hold **nearly
identical columns**, differing only by `address`. Merging them — PK extended
with `address`, one row per (event × participant) — removes that duplication.

This is sound because of two measured facts (mainnet, last 100 blocks, the real
flatten + `extract_addresses` + system-contract blacklist):

- **Most events are address-less system noise** that the **event-type
  blacklist** (§ Configuration) drops anyway: `guest` 47%, `withhold` /
  `authenticate` / `finalize` ~11% each, leaving `contract_event` at ~6%. After
  the default blacklist the surviving stream is ~**81% smaller**, and what
  remains is mostly the meaningful, address-bearing events.
- **Fan-out is tiny: a contract event carries ~1 non-system address** (the
  emitting contract is blacklisted as a participant, leaving the user). So one
  event ≈ one row — the "K rows per event" cost is ~1×.

Trade-offs the merge accepts (and why they're fine here):

- A kept event with **no** participant (e.g. an `execute`) still needs a row so
  the attribute feeds (queries 3/6) don't lose it. It gets **one row keyed by
  the empty byte string** — a real address is always 20 bytes, so `''` can never
  collide, and (unlike `NULL`) an empty value is valid in a primary key.
- The address-less attribute feeds (queries 3/4/6) must `DISTINCT ON` the event
  position to collapse an event's K participant rows. At K≈1 this is a **no-op**;
  it stays correct for K>1 (multi-recipient transfers). The `address` column is
  the **last** index column so those K rows are adjacent and the dedup is local.

Net effect, per 100 blocks (measured, default blacklist): the merge saves ~**24%**
of event/involvement rows versus the two-table model (and up to 50% if the feed
is restricted to address-bearing types) — on top of the ~81% the blacklist
already removes. The cost is the `DISTINCT ON` and a deliberate empty-address
marker.

## Backend: Postgres — and when ClickHouse

All three tables are on **Postgres** for V1: fixed access patterns on **real
columns** (`event_type`, `contract`, `contract_event_name`, `address`) — ordinary
composite B-tree territory with keyset pagination, native joins, and an
exactly-once cursor in the same transaction as the data. One database keeps V1
simple.

This is a large dataset, but the **event-type blacklist** is the dominant size
lever: the raw stream is ~112 flattened events/block, of which ~81% is dropped
noise, leaving ~20/block of meaningful, mostly address-bearing events. Postgres
serves the *queries* fine (every one is a bounded keyset scan on a good index),
so the cost center is **storage and write throughput**, addressed by: the
blacklist, the payload split (below), the involvement blacklist (below),
`block_height`-range partitioning, and offline backfill.

**ClickHouse is the documented escape hatch.** If volume outgrows Postgres, the
`events` read-path moves to ClickHouse: the filter columns are low-cardinality
and compress ~an order of magnitude better columnar, alternate sort orders are
cheap (projections), and the committer already supports a ClickHouse side **in
the same commit** (flushed + acked before the PG transaction, idempotent by a
per-unit dedup token) — so it is a per-table migration, not a rearchitecture. We
stay Postgres-only until a measured trigger says otherwise.

## Identity: the position, not the hash

The canonical id of a unit is its **position** `(block_height, idx, kind)`; of
an event, `(block_height, category, category_index, event_index)`; of a row in
`events`, that event position **plus `address`**. The transaction **hash is a
non-unique, indexed attribute** — never a key.

The hash is not globally unique. Uniqueness would have to lean on the nonce,
and it doesn't:

- **Failed txs don't consume their nonce.** In `process_tx`
  (`dango/core/app/src/app.rs`), a tx that fails `withhold_fee` (sender can't
  cover the max fee) or `authenticate` returns a failed outcome while the state
  buffer holding the nonce write into `SEEN_NONCES` is **dropped, not
  committed** — yet the tx is still recorded in the block
  (`block_outcome.tx_outcomes[i]`, aligned by index with `block.txs[i]`). So
  identical bytes can be re-included validly in a **later** block.
- **Some senders have no nonce.** A smart contract can be a tx `sender` through
  its own `authenticate` entrypoint; those txs have no nonce at all.

Same bytes ⇒ same `Hash256`, in two different blocks. Within one block it can't
repeat (the mempool dedups by hash). So the position is unique by construction
and the hash is just an indexed column — a lookup by hash may legitimately
return more than one row.

## Tables

### `transactions` — one row per executed unit

| column         | type            | notes                                   |
|----------------|-----------------|-----------------------------------------|
| `block_height` | `BIGINT`        |                                         |
| `idx`          | `INT`           | position within block, per kind         |
| `kind`         | `SMALLINT`      | the unit's `FlatCategory`: 0 = cron, 1 = tx |
| `hash`         | `BYTEA` (32)    | content hash; indexed, **not unique**; NULL for cron |
| `sender`       | `BYTEA` (20)    | account **or contract**; NULL for cron  |
| `success`      | `BOOL`          | `tx_outcome.result.is_ok()`             |
| `gas_limit`    | `BIGINT`        | NULL for cron (unlimited)               |
| `gas_used`     | `BIGINT`        |                                         |
| `timestamp`    | `BIGINT`        | block time, unix nanoseconds            |

Primary key `(block_height, idx, kind)`. Indexes: `(sender, block_height, idx)`
partial `WHERE sender IS NOT NULL` — query 2, recency pagination via backward
scan, excludes cron; `(hash)` partial `WHERE hash IS NOT NULL` — hash lookup.

No payload column: a unit's messages / credential / error are **hydrated from
the raw block** on the detail view (one block load — see below).

### `events` — merged event log + participation index

One row per **(event × participant address)**. Three row kinds, all keyed by
`(address, block_height, category, category_index, event_index)`:

| column                | type         | notes                                   |
|-----------------------|--------------|-----------------------------------------|
| `address`             | `BYTEA`      | 20-byte participant; **empty** = address-less event; sender on the sentinel |
| `block_height`        | `BIGINT`     |                                         |
| `category`            | `SMALLINT`   | the unit's `kind` (`FlatCategory`): 0 = cron, 1 = tx |
| `category_index`      | `INT`        | the tx/cron index (= the unit's `idx`)  |
| `event_index`         | `INT`        | event position within the unit (0-based); **`-1`** = sentinel |
| `event_type`          | `SMALLINT`   | `FlatEvent` discriminant; NULL only on the sentinel |
| `contract`            | `BYTEA` (20) | emitting / subject contract; NULL where not applicable |
| `contract_event_name` | `TEXT`       | the contract-event `ty` (order_filled, …); NULL otherwise |

- **participation** — `address` = a 20-byte party, `event_index >= 0`. K rows
  for an event with K participants (K ≈ 1).
- **address-less event** — `address` = the **empty byte string**,
  `event_index >= 0`. One per kept event with no (non-blacklisted) participant,
  so the attribute feeds never lose it. (A real address is always 20 bytes, so
  `''` never collides; `NULL` would be illegal in a PK.)
- **sentinel** — `address` = the tx sender, `event_index = -1`,
  `event_type`/`contract`/`contract_event_name` = NULL. Lets "txs involving X"
  surface a unit where X is only the sender.

The canonical *event* is the position `(block_height, category, category_index,
event_index)`; `(block_height, category_index, category)` joins
`transactions(block_height, idx, kind)`. Distinct events are recovered with
`DISTINCT ON` the position (a no-op at K = 1).

**Indexes** (six, all partial `WHERE <attr> IS NOT NULL`, which also skips the
sentinel). Tail is the event position `(block_height, category, category_index,
event_index)`; `address` is last on the attribute-only trio so a multi-party
event's rows are adjacent for the `DISTINCT ON`.

- `(address, contract, …)` · `(address, contract_event_name, …)` ·
  `(address, event_type, …)` — **query 7**: "<attr> events involving X" as a
  single seek (e.g. the gateway/perps history of X, `order_filled` involving X).
- `(event_type, …, address)` · `(contract, …, address)` ·
  `(contract_event_name, …, address)` — **queries 3/4/6**: the address-less
  attribute feeds (backward scan + `DISTINCT ON`).

The PK `(address, …)` itself serves **query 5** (events involving X) and, via
`DISTINCT ON` the unit, **query 1** (txs involving X).

### `event_data` — the event payload, split out

| column           | type       | notes                |
|------------------|------------|----------------------|
| `block_height`   | `BIGINT`   |                      |
| `category`       | `SMALLINT` |                      |
| `category_index` | `INT`      |                      |
| `event_index`    | `INT`      |                      |
| `data`           | `BYTEA`    | `zstd(borsh(event))` |

Primary key `(block_height, category, category_index, event_index)` — the event
position. A row exists **only** for priority-type events; a missing row means
"hydrate the payload from the raw block". No secondary indexes — reached only by
point lookup on the detail view. Splitting the payload out keeps the hot,
frequently-scanned `events` rows fixed-width and index-dense (the cold,
variable-size blob lives apart): narrower rows, smaller indexes, better cache.

## Access paths

All newest-first, keyset-paginated (no `OFFSET`).

**Transactions where X is the sender** (query 2) — directly on `transactions`,
served by `(sender, block_height, idx)`:

```sql
SELECT * FROM transactions
WHERE sender = $X
ORDER BY block_height DESC, idx DESC
LIMIT 20;                       -- next page: AND (block_height, idx) < ($bh, $idx)
```

**Transactions involving X** (query 1) — `DISTINCT ON` the unit over X's rows,
then join. The PK serves it as a backward Index Scan → Unique → Limit, short-
circuiting at 20 distinct units:

```sql
SELECT t.*
FROM (
    SELECT DISTINCT ON (block_height, category_index, category)
           block_height, category_index, category
    FROM events
    WHERE address = $X
    ORDER BY block_height DESC, category_index DESC, category DESC, event_index DESC
    LIMIT 20
) u
JOIN transactions t
  ON t.block_height = u.block_height AND t.idx = u.category_index AND t.kind = u.category
ORDER BY t.block_height DESC, t.idx DESC, t.kind DESC;
```

**Events involving X** (query 5), optionally filtered by an attribute (query 7)
— a single seek on the PK or an `(address, <attr>, …)` index, no `DISTINCT`
(X appears once per event):

```sql
SELECT * FROM events
WHERE address = $X AND event_index >= 0          -- AND contract = $C  (query 7)
ORDER BY block_height DESC, category DESC, category_index DESC, event_index DESC
LIMIT 20;
```

**Events by type / name / contract** (queries 3, 4, 6, no address) — on the
`(<attr>, …, address)` indexes, with `DISTINCT ON` the position to collapse an
event's participant rows (a no-op at K = 1):

```sql
SELECT DISTINCT ON (block_height, category, category_index, event_index) *
FROM events
WHERE contract_event_name = $N                   -- or event_type / contract
ORDER BY block_height DESC, category DESC, category_index DESC, event_index DESC, address
LIMIT 20;
```

### Index summary — every documented query is anchored by a selective seek

| Query | Index | Cost |
|---|---|---|
| tx where sender = X | `transactions (sender, bh, idx)` | seek + 20 |
| tx involving X | `events` PK → `DISTINCT ON` units | seek + 20 units |
| events involving X | `events` PK `(address, bh, …)` | seek + 20 |
| **C / N / T events involving X** | `events (address, contract\|name\|type, …)` | **seek + 20** |
| events of type T / contract C / name N | `events (<attr>, …, address)` + `DISTINCT ON` | seek + ~K·20 |
| event payload (detail) | `event_data` PK, else raw block | point lookup |

No query scans the whole table — each is anchored on a selective equality
(sender / address / contract / name) then a `block_height`-ordered scan, so the
cost is bounded by **one entity's activity**, not the table total. The
attribute-only feeds read ~K index entries per emitted event (K ≈ 1), all
sequential; the heap reads (the cost) equal the page size. **The query layer
must write these as `DISTINCT ON (<position>) … ORDER BY <position>, address …
LIMIT n`** so Postgres streams + short-circuits; a non-matching `ORDER BY`
materializes the whole match set instead.

## What is stored vs hydrated from the raw block

The raw block is always reachable through `source.get(height)` — in V1 a single
page-cached read of the node's on-disk cache file, the source of truth for
payloads. The projection stores the **queryable index**, not a second copy.

- **Stored in PG:** the three tables above; the event payload (`event_data`)
  only for the configured priority types.
- **Hydrated from the raw block on demand:** a transaction's full
  payload/messages/error (the tx detail view), and the payload of any event with
  no `event_data` row (non-priority types).

The split follows the numbers. A **tx detail** is block-local: one `source.get`
returns the tx and *all* its events — cheap. An **event feed**, filtered and
ordered across blocks, hits ~one matching event per block, so a page would
otherwise mean ~one cold-block read per row — wasteful and SLA-breaking on
historical (cold) data. So the priority payload lives in `event_data` and the
feed never hydrates; only the rare detail of a non-priority event pays one
block load.

## Per-block processing

`process(&mut ctx, block)` flattens the block's events once (tx units via
`flatten_tx_events`, cron units via `flatten_commitment_status` on a fresh
`EventId`), then for each unit:

For each `tx_outcome[i]` aligned with `block.txs[i] = (tx, hash)`:

1. write one `transactions` row (`kind = tx`, `sender`, gas, success, `hash`,
   `timestamp`);
2. write the **sentinel** `events` row (`address = sender`, `event_index = -1`,
   NULL attributes);
3. for each flattened event of the tx:
   - **skip it entirely** if its type is in `event_type_blacklist`;
   - if its type is a priority type, write its `event_data` row;
   - extract participants for the `involvement_types` (the chain's
     `Extractable::extract_addresses`, **minus the blacklist**); write one
     `events` row per participant, each carrying `event_type` / `contract` /
     `contract_event_name`. If there is **no** participant, write a single
     empty-address row so the event still appears in the attribute feeds.

For each `cron_outcome` (same block): the same, with `kind = cron`,
`sender = NULL`, no sentinel row.

Why two separate scopes — `event_type_blacklist` (drop the event) vs
`involvement_types` (extract participants): the blacklist removes the
address-less system *noise* (guest/withhold/authenticate/finalize) that nobody
queries; `involvement_types` decides, among the **kept** events, which ones fan
out by participant (the rest get one empty-address row). "Events emitted **by**
a contract" (query 6) read `events.contract`, a different axis from
participation — so blacklisting a contract from *participation* never hides its
emitted events.

## Configuration

| key                    | default                          | effect                                                          |
|------------------------|----------------------------------|-----------------------------------------------------------------|
| `event_type_blacklist` | `[guest, withhold, authenticate, finalize, backrun, reply]` | event types dropped entirely (no `events` row, no payload) — the address-less system noise (~81% of the raw stream) |
| `event_data_types`     | `[transfer, contract_event]`     | event types whose payload is stored in `event_data`; others lazy-load from the raw block |
| `involvement_types`    | `[transfer, contract_event]`     | among kept events, which fan out by participant (others get one empty-address row) |
| `involvement_blacklist`| system contracts                 | addresses excluded from participation at write time (perps, taxman, oracle, bank, dex, account_factory, gateway, warp, hyperlane.*) |

Maintenance note: all applied at **write time**, so changes are not retroactive.
*Narrowing* `event_type_blacklist`, adding to `event_data_types`, broadening
`involvement_types`, or *removing* a `involvement_blacklist` entry requires a
re-backfill (drop tables + cursor, bump the projection id, re-run) to populate
the now-wanted rows; *widening* the blacklist leaves already-written rows until a
targeted `DELETE`. Prefer settling them before the first full backfill.

## Commit / idempotency

Every key is the deterministic position (+ `address`), and every write is an
upsert (`INSERT … ON CONFLICT DO NOTHING` on the primary keys), so a post-crash
replay of a block is a no-op. All three tables plus the cursor commit in **one
Postgres transaction** — the historical indexer's PG-side exactly-once. No
ClickHouse path in V1.

## Crate layout

`projection/src/activity/`: `mod.rs` (the `Projection` impl + `ActivityConfig`,
`id = "activity"`, `min_height = 0`) · `event_type.rs` (the `EventType`
discriminant stored in `event_type`, plus the per-event contract / name
taxonomy) · `entity/` (sea-orm types for the three tables) · `idens.rs`
(migration identifiers) · `migrations/` (one file per table + a `mod.rs`
assembling them, names prefixed `…activity…` for the shared `seaql_migrations`
history — mirrors `app/src/committer/`). Adds `dango-primitives` to the
`projection` crate (for `FlatCategory` / `EventId` / `Extractable` /
`flatten_tx_events` / `flatten_commitment_status`), plus `zstd`/`borsh` for the
`event_data` payloads.

Flattening note: tx units use the canonical `flatten_tx_events`; cron units use
`flatten_commitment_status` on a fresh per-unit `EventId`. Both number
`event_index` from 0 within the unit, and every leaf self-advances the index
(`next_id.event_index += 1`), so a unit's commitment groups stay contiguous and
never collide — no explicit `increment_idx` normalization is needed at this
layer. Encoding note: `category` / `kind` store the canonical `FlatCategory`
discriminant (cron = 0, tx = 1) directly; the integer value is immaterial to the
queries as long as `events.category` and `transactions.kind` agree on it for the
join.

## Out of scope

The query / front-door layer (REST / GraphQL / gRPC — undecided) and pagination
cursors. The tables here fix the storage model those queries will run on.
