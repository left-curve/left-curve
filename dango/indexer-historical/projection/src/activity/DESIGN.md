# Activity projection — design

The foundational projection of the historical indexer. For every block it
produces three Postgres tables that together answer, for any account address,
**"what did it do and what touched it"**:

- `transactions` — one row per executed unit (a transaction *or* a cronjob);
- `events` — one row per flattened event (the queryable event log);
- `involvement` — an address → event/unit participation index.

It is a **single projection**: one `process(block)` flattens the block's
events once and writes all three tables in one commit (one `Ctx`, one cursor).

## Queries it must serve

All paginated **newest-first** (`block_height DESC`, then in-block position),
keyset-paginated (cursor on the ordering tuple, never `OFFSET`):

1. transactions involving an address X (X is the sender, or a party in one of
   the unit's events);
2. transactions where X is specifically the sender;
3. events filtered by **event type** (e.g. `transfer`, `contract_event`);
4. contract events filtered by **name** (e.g. `order_filled`, `liquidated`,
   `dispatched`);
5. events where an address X **participates**;
6. (bonus, free from the schema) events emitted **by a contract** C.

## Backend: Postgres — and when ClickHouse

All three tables are on **Postgres** for V1. The access patterns are fixed and
expressed on **real columns** (`event_type`, `contract`, `contract_name`) plus
an address join table — i.e. ordinary composite B-tree territory with keyset
pagination, native joins, and an exactly-once cursor in the same transaction as
the data. One database keeps V1 simple.

This is a large dataset. Measured on mainnet (sample of recent blocks):
**~9 transactions/block** and **~57 flattened events/block** (p50 47, p99 ~143,
max ~239), of which **~8 are contract events**. At ~30M blocks in the first ~6
months (≈2 blocks/s) that is on the order of **~1.7B event rows** and **~1B
involvement rows**, growing ~3.4B events/year. Postgres serves the *queries*
fine (every one is a bounded keyset scan on a good index), so the cost center
is **storage and write throughput**, addressed by: partitioning the big tables
by `block_height` range, the data/lazy-load split (below), the involvement
blacklist (below), and offline backfill.

**ClickHouse is the documented escape hatch.** If, at this volume, Postgres
storage or tail latency stops being sustainable, the natural move is to push
the `events` + `involvement` read-path to ClickHouse: the filter columns are
low-cardinality and compress roughly an order of magnitude better columnar, and
alternate sort orders are cheap there (projections). The historical indexer's
committer already supports a ClickHouse side **in the same commit** (ClickHouse
flushed and acked before the Postgres transaction, made idempotent by a
per-unit deduplication token), so the migration is per-table, not a
rearchitecture. We stay Postgres-only until a measured trigger (storage growth
per access pattern, or p99 breaking despite tuning) says otherwise.

## Identity: the position, not the hash

The canonical id of a unit is its **position** `(block_height, idx, kind)`; of
an event, `(block_height, category, category_index, event_index)`. The
transaction **hash is a non-unique, indexed attribute** — never a key.

The hash is not globally unique. Uniqueness would have to lean on the nonce,
and it doesn't:

- **Failed txs don't consume their nonce.** In `process_tx`
  (`dango/core/app/src/app.rs`), a tx that fails `withhold_fee` (sender can't
  cover the max fee) or `authenticate` returns a failed outcome while the state
  buffer holding the nonce write into `SEEN_NONCES` is **dropped, not
  committed** — yet the tx is still recorded in the block
  (`block_outcome.tx_outcomes[i]`, aligned by index with `block.txs[i]`). So
  identical bytes (insufficient balance → later funded; or a nonce briefly
  outside the acceptance window) can be re-included validly in a **later**
  block.
- **Some senders have no nonce.** A smart contract can be a tx `sender` through
  its own `authenticate` entrypoint; those txs have no nonce at all.

Same bytes ⇒ same `Hash256`, in two different blocks. Within one block it can't
repeat (the mempool dedups by hash). So the position is unique by construction
and the hash is just an indexed column — a lookup by hash may legitimately
return more than one row. Any other projection that processes the same block
knows both handles for free (`block.txs[i].1` is the hash, `i`/`kind` the
position), so referencing a unit needs no database lookup either way.

## Tables

### `transactions` — one row per executed unit

| column         | type            | notes                                   |
|----------------|-----------------|-----------------------------------------|
| `block_height` | `BIGINT`        |                                         |
| `idx`          | `INT`           | position within block, per kind         |
| `kind`         | `SMALLINT`      | 0 = tx, 1 = cron                        |
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

### `events` — one row per flattened event (the full event log)

| column           | type         | notes                                            |
|------------------|--------------|--------------------------------------------------|
| `block_height`   | `BIGINT`     |                                                  |
| `category`       | `SMALLINT`   | 0 = tx, 1 = cron (= the unit's `kind`)          |
| `category_index` | `INT`        | the tx/cron index (= the unit's `idx`)          |
| `event_index`    | `INT`        | event position within the unit                  |
| `event_type`     | `SMALLINT`   | `FlatEvent` discriminant (transfer, contract_event, execute, withhold, …) |
| `contract`       | `BYTEA` (20) | emitting contract; NULL where not applicable    |
| `contract_name`  | `TEXT`       | the contract-event `ty` (order_filled, …); NULL otherwise |
| `data`           | `BYTEA`      | `zstd(borsh(event))`; **NULL for non-priority types** (lazy-loaded) |

Primary key `(block_height, category, category_index, event_index)` — the
positional identity, also "events in a block", dedup/idempotency, and the
**reference to the unit**: `(block_height, category_index, category)` joins
`transactions(block_height, idx, kind)`. No separate `tx_id` column.

Indexes: PK; `(event_type, block_height, category, category_index, event_index)`
— query 3; `(contract_name, block_height, …)` partial `WHERE contract_name IS
NOT NULL` — query 4; `(contract, block_height, …)` partial `WHERE contract IS
NOT NULL` — query 6.

**Every flattened event gets a row** (so the involvement join can never
orphan), but `data` is populated only for the configured *priority* event types
(§ Configuration); for the rest it is `NULL` and reconstructed from the raw
block on demand.

### `involvement` — address → event participation

| column           | type         | notes                                   |
|------------------|--------------|-----------------------------------------|
| `address`        | `BYTEA` (20) |                                         |
| `block_height`   | `BIGINT`     |                                         |
| `category`       | `SMALLINT`   |                                         |
| `category_index` | `INT`        | the unit                                |
| `event_index`    | `INT`        | the event; **sentinel `-1`** = unit-level (sender) |

Primary key `(address, block_height, category_index, category, event_index)` —
`category_index` precedes `category` so the recency order
`(block_height DESC, category_index DESC)` is a backward index scan with no sort
(see § Access paths).

One table serves both the event feed and the tx feed:

- **events involving X** (query 5) → `WHERE address = X AND event_index >= 0` →
  join `events`. One row per event (no dedup). Backward scan = newest-first.
- **transactions involving X** (query 1) → `DISTINCT ON
  (block_height, category_index, category)` over `WHERE address = X`, then join
  `transactions`. The `LIMIT` sits on the **distinct units**, never the rows
  (several event rows can share one unit). The `event_index = -1` **sentinel
  row** carries the sender, so a unit appears even when no participation-type
  event involves X (e.g. X called a contract that emitted events about others).

This is the event-level address index *subsuming* the unit-level one — a single
fan-out table instead of two. Because granularity is per-event, the same
address can have several rows for one unit; the tx feed collapses them. `BYTEA`
fixed widths (address 20, never hex text) keep this — the largest table — lean.

## Access paths

All newest-first, keyset-paginated (no `OFFSET`).

**Transactions where X is the sender** (query 2) — directly on `transactions`,
one row per tx, served by `(sender, block_height, idx)`:

```sql
SELECT * FROM transactions
WHERE sender = $X
ORDER BY block_height DESC, idx DESC
LIMIT 20;
-- next page: AND (block_height, idx) < ($bh, $idx)
```

**Transactions involving X** (query 1) — the `LIMIT` goes on the distinct
units, so `DISTINCT ON` first, then join. The `involvement` primary key serves
it as a backward Index Scan → Unique → Limit, which short-circuits at 20
distinct units (it does not scan all of X's history):

```sql
SELECT t.*
FROM (
    SELECT DISTINCT ON (block_height, category_index, category)
           block_height, category_index, category
    FROM involvement
    WHERE address = $X
    ORDER BY block_height DESC, category_index DESC, category DESC, event_index DESC
    LIMIT 20
) u
JOIN transactions t
  ON t.block_height = u.block_height AND t.idx = u.category_index AND t.kind = u.category
ORDER BY t.block_height DESC, t.idx DESC, t.kind DESC;
-- next page: AND (block_height, category_index, category) < ($bh, $ci, $cat)
```

**Events involving X** (query 5) — one row per event, no dedup, join `events`:

```sql
SELECT e.*
FROM involvement i
JOIN events e USING (block_height, category, category_index, event_index)
WHERE i.address = $X AND i.event_index >= 0
ORDER BY i.block_height DESC, i.category_index DESC, i.category DESC, i.event_index DESC
LIMIT 20;
```

**Events by type / name / contract** (queries 3, 4, 6) — directly on `events`
via `(event_type, block_height, …)`, `(contract_name, block_height, …)` partial,
and `(contract, block_height, …)` partial; same backward-scan + keyset shape.

## What is stored vs hydrated from the raw block

The raw block (every tx + the full event tree) is always reachable through
`source.get(height)` — in V1 a single page-cached read of the node's on-disk
cache file. It is the source of truth for payloads, so projections store the
**queryable index**, not a second copy of everything.

- **Stored in PG:** the three tables above, plus event `data` for the
  configured priority types only.
- **Hydrated from the raw block on demand:** a transaction's full
  payload/messages/error (the tx detail view), and the `data` of non-priority
  events.

The split follows the numbers. A **tx detail** is block-local: one
`source.get` returns the tx and *all* its events — cheap. An **event feed**,
filtered and ordered across blocks, hits ~one matching event per block, so a
page of 50 would otherwise mean ~50 cold-block reads, each decompressing and
flattening ~57 events to use one — wasteful and SLA-breaking on historical
(cold) data. So the priority event `data` lives in the table and the feed never
hydrates; only the rare detail of a non-priority event pays one block load.

## Per-block processing

`process(&mut ctx, block)` flattens the block's events once, then for each unit:

For each `tx_outcome[i]` aligned with `block.txs[i] = (tx, hash)`:
1. write one `transactions` row (`kind = tx`, `sender = tx.sender`, gas,
   success from `result`, `hash`, `timestamp` from `block.info`);
2. for each flattened event of the tx: write one `events` row (`event_type`,
   `contract`, `contract_name`, and `data` iff its type is a priority type);
3. write `involvement` rows: the **sender** as the `event_index = -1` sentinel,
   plus, for each event whose type is in `involvement_types`, every address the
   chain's `Extractable::extract_addresses` yields for it — **minus the
   blacklist**, at write time.

For each `cron_outcome` (same block): the same, with `kind = cron`,
`category_index` = the cron's index, `sender = NULL`, no sentinel row.

Why scope the involvement *extraction* (not the `events` table) to
`involvement_types`: every flattened event still gets an `events` row, but
extracting participants from system events (withhold/authenticate/execute
wrappers) only ever re-yields the sender (already captured by the sentinel) and
system contracts (blacklisted) — pure redundant rows. So by default we extract
participation from the meaningful types (`transfer`, `contract_event`);
broaden it via config only if a real need appears.

"Events emitted **by** a contract" (query 6) come from `events.contract`, a
different axis from participation — so blacklisting a contract from
`involvement` never hides its emitted events.

## Configuration

| key                    | default                      | effect                                                            |
|------------------------|------------------------------|-------------------------------------------------------------------|
| `event_data_types`     | `[transfer, contract_event]` | event types whose `data` is stored in PG; others store `NULL` and lazy-load |
| `involvement_types`    | `[transfer, contract_event]` | event types whose participants enter `involvement`                |
| `involvement_blacklist`| system contracts             | addresses excluded from `involvement` at write time (perps, taxman, oracle, bank, dex, account_factory, gateway, warp, hyperlane.*) |

Maintenance note: these are applied at **write time**, so changing them is not
retroactive. Adding a type to `event_data_types`, broadening `involvement_types`,
or *removing* an address from the blacklist requires re-backfilling the
projection (drop tables + cursor, bump the projection id, re-run) to populate
the now-wanted rows; *adding* a blacklist entry leaves its already-written rows
until a targeted `DELETE`. Prefer settling them before the first full backfill.

## Commit / idempotency

Every key is the deterministic position, and every write is an upsert
(`INSERT … ON CONFLICT DO NOTHING` on the positional primary keys), so a
post-crash replay of a block is a no-op. All three tables plus the cursor commit
in **one Postgres transaction** — the historical indexer's PG-side
exactly-once. No ClickHouse path in V1.

## Planned crate layout

`projection/src/activity/`: `mod.rs` (the `Projection` impl, `id = "activity"`,
`min_height = 0`) · `entity/` (sea-orm types for the three tables) · `idens.rs`
(migration identifiers) · one migration file per table (names prefixed
`…activity…` for the shared `seaql_migrations` history). Adds `dango-primitives`
to the `projection` crate (for `Block`/`BlockOutcome`/`flatten_tx_events`/
`Extractable`), plus `zstd`/`borsh` for the `data` column.

## Out of scope

The query / front-door layer (REST / GraphQL / gRPC — undecided) and pagination
cursors. The tables here fix the storage model those queries will run on.
