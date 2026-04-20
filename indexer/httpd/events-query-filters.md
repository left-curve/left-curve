# Spec — Filters on the generic `events` GraphQL query

**Status:** draft
**Scope:** `indexer/httpd` (generic indexer GraphQL layer)
**Related code:**
- `indexer/httpd/src/graphql/query/event.rs`
- `indexer/httpd/src/graphql/subscription/event.rs`
- `indexer/httpd/src/graphql/query/pagination.rs`
- `indexer/sql/src/entity/events.rs`
- `indexer/sql/src/active_model.rs`
- `indexer/sql-migration/src/m20250717_192805_events_data_index.rs`

## 1. Motivation

Today the generic `events` query supports only cursor pagination (Relay-style).
Filtering is only available on the `events` subscription via a `Filter` input
object (type + JSONB containment on `data`).

Consumers who need to page through historical events of a specific kind — e.g.
all `dispatch_v2` contract events emitted by a given Hyperlane mailbox, or all
`OrderFilled` events of a specific user on the dex — have no usable path:

- The query returns every event unfiltered, so clients must fetch everything
  and filter client-side (wasteful).
- The subscription supports filters but is streaming and caps historical
  backfill at `MAX_PAST_BLOCKS = 100` blocks.

The Dango httpd merges `IndexerQuery` from the generic httpd into its own
schema, so extending the generic query benefits both the generic and the Dango
GraphQL endpoints without touching the Dango crate.

The core constraint driving this spec: the `events` table is expected to grow
into the **billions of rows**. The endpoint must stay within a **500 ms budget
per request** and must not be exploitable as a DoS vector.

## 2. Current state

### 2.1 `EventQuery::events` — `query/event.rs`

```rust
async fn events(
    ctx,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    sort_by: Option<SortBy>,
) -> Connection<OpaqueCursor<EventCursor>, events::Model>
```

Cursor keyed on `(block_height, event_idx)`. Max 100 rows per page. No content
filters.

### 2.2 `EventSubscription::events` — `subscription/event.rs`

```rust
Filter     { type: Option<String>, data: Option<Vec<FilterData>> }
FilterData { path: Vec<String>, checkMode: EQUAL|CONTAINS, value: Vec<Json> }
```

Semantics:
- Outer list of `Filter`s combined with **OR**.
- Inside one `Filter`, `type` and every `FilterData` check combined with **AND**.
- `FilterData` is rendered as a JSONB containment (`data @> {...nested...}`).

### 2.3 Event row shape and existing indexes

Relevant columns on `events`:

| column | type | notes |
| --- | --- | --- |
| `type` | `string` | `FlatEvent` variant name, snake_case (`transfer`, `execute`, `contract_event`, …) |
| `method` | `string?` | parsed from the inner JSON object |
| `data` | `jsonb` | serialized `FlatEvent` with the external variant tag (see §2.4) |
| `block_height` | `i64` | |
| `event_idx` | `i32` | per-block ordinal |

Existing indexes:

- `events-block_height` — btree on `block_height`.
- `events-data` — GIN on `data`. The migration uses sea-query `.full_text()`;
  sea-query 0.32.7 translates this on Postgres to `USING GIN` without an
  explicit operator class, so Postgres falls back to the default `jsonb_ops`.
  `jsonb_ops` accelerates `@>`, `?`, `?|`, `?&` — all of which we either use
  today (`@>`) or may use in the future. A migration to `jsonb_path_ops` would
  be marginally smaller/faster for `@>`-only workloads but is not a blocker.

### 2.4 What ends up in `events.data`

`indexer/sql/src/active_model.rs` calls `serde_json::to_value(&index_event.event)`
on the `FlatEvent` and stores the result verbatim. The "removing the top hash"
comment in that function is stale: `inside_data` is computed only to extract
`method`, the column stores the full externally-tagged JSON.

`FlatEvent` is serde-tagged `rename_all = "snake_case"` with the default
external-tag representation. For a contract event the stored JSON is:

```json
{
  "contract_event": {
    "contract": "<emitter_addr>",
    "type": "<contract_event_name>",
    "data": { /* event payload */ }
  }
}
```

Consequence: **every custom event emitted via `Response::add_event` has
`events.type = "contract_event"`** in the DB. The Rust-level event name lives
at `data.contract_event.type`.

## 3. Proposed API

Extend `EventQuery::events` with three new arguments, all optional:

```graphql
events(
  after: String
  before: String
  first: Int
  last: Int
  sortBy: EventSortBy

  # New
  filter: [EventFilter!]
  fromBlockHeight: Int
  toBlockHeight: Int
): EventConnection!

input EventFilter {
  type: String
  data: [EventFilterData!]
}

input EventFilterData {
  path: [String!]!
  checkMode: EventFilterCheckMode!
  value: [JSON!]!
}

enum EventFilterCheckMode { EQUAL CONTAINS }
```

Semantics (reused from the subscription, no new grammar introduced):
- `filter` — outer list OR'd, inner `type` + `data` AND'd. Omitted → no
  content filtering.
- `fromBlockHeight` — inclusive lower bound on `block_height`. Omitted → no
  lower bound.
- `toBlockHeight` — inclusive upper bound on `block_height`. Omitted → no
  upper bound.

All three compose in `AND` with each other, with the cursor clause, and with
the `ORDER BY`.

Requests must additionally pass the admission rules in §4.4. At runtime every
query is bounded by the per-transaction `statement_timeout` set in §4.5.

### 3.1 Example — all `OrderFilled` events of a user on the dex

```graphql
query {
  events(
    first: 100
    sortBy: BLOCK_HEIGHT_DESC
    fromBlockHeight: 1000000
    toBlockHeight: 2000000
    filter: [{
      type: "contract_event"
      data: [
        { path: ["contract_event", "contract"], checkMode: EQUAL,
          value: ["grug1dex..."] },
        { path: ["contract_event", "type"],     checkMode: EQUAL,
          value: ["OrderFilled"] },
        { path: ["contract_event", "data", "user"], checkMode: EQUAL,
          value: ["grug1alice..."] }
      ]
    }]
  ) {
    pageInfo { hasNextPage endCursor }
    edges {
      cursor
      node { type data blockHeight createdAt }
    }
  }
}
```

## 4. Implementation plan

### 4.1 Extract shared filter module

Move from `subscription/event.rs` to a new `graphql/event_filter.rs`:

- Rust types renamed to `EventFilter`, `EventFilterData`, `EventFilterCheckMode`.
  The subscription's public GraphQL schema is preserved via
  `#[graphql(name = "Filter")]` / `"FilterData"` / `"CheckValue"` aliases on the
  existing input objects, so existing subscription clients are unaffected. The
  new query resolver uses the new names directly.
- Parsed internals: `ParsedFilter`, `ParsedFilterData`, `ParsedCheckValue`.
- `parse_filter(Vec<Filter>) -> Result<Vec<ParsedFilter>>` — preserves the
  existing EQUAL-must-have-1-value / CONTAINS-must-have-≥1-value validation,
  plus the admission caps from §4.4.
- New: `build_filter_expr(filters: Vec<ParsedFilter>) -> Option<SimpleExpr>` —
  contains exactly the current logic from `precompute_query` that builds
  `or_filter_expr`, minus the `Entity::find().order_by(...)` part.

### 4.2 Adapt subscription

`precompute_query` becomes a thin adapter:

```rust
let mut q = Entity::find()
    .order_by_asc(Column::BlockHeight)
    .order_by_asc(Column::EventIdx);
if let Some(expr) = build_filter_expr(filters) {
    q = q.filter(expr);
}
q
```

Behavior and GraphQL API of the subscription stay identical.

### 4.3 Extend the query resolver

```rust
async fn events(
    &self,
    ctx,
    after, before, first, last,
    sort_by: Option<SortBy>,
    filter: Option<Vec<Filter>>,
    from_block_height: Option<u64>,
    to_block_height: Option<u64>,
) -> Result<Connection<...>> {
    let parsed = filter.map(parse_filter).transpose()?;

    // §4.4 — reject shapes that cannot be served within budget
    admission_check(&parsed, from_block_height, to_block_height)?;

    let expr = parsed.and_then(build_filter_expr);

    paginate_models(
        app_ctx, after, before, first, last, sort_by, 100,
        move |mut query, txn| {
            Box::pin(async move {
                // §4.5 — per-request Postgres budget
                txn.execute_unprepared("SET LOCAL statement_timeout = '500ms'")
                    .await?;
                if let Some(e) = expr { query = query.filter(e); }
                if let Some(f) = from_block_height {
                    query = query.filter(Column::BlockHeight.gte(f as i64));
                }
                if let Some(t) = to_block_height {
                    query = query.filter(Column::BlockHeight.lte(t as i64));
                }
                Ok(query)
            })
        },
    ).await
}
```

No changes to `paginate_models`, `EventCursor`, or `SortBy`. All new
predicates compose with the existing `cursor_filter`/`cursor_order` via
plain `AND`.

### 4.4 Admission rules (mandatory)

These validate the input **before** it hits the database. The goal is that any
request passing admission can complete within the `statement_timeout` budget
even on a billions-of-rows table.

**Filter shape caps.** Reject payloads that exceed:

| Limit | Default | Rationale |
| --- | --- | --- |
| `filter.len()` | 10 | Each extra OR'd branch multiplies plan cost. |
| `filter[i].data.len()` | 5 | Bounded WHERE size. |
| `filter[i].data[j].value.len()` (CONTAINS) | 20 | Each value becomes an OR'd `@>` clause. |
| `filter[i].data[j].path.len()` | 10 | Prevents absurdly deep containment objects. |

Conservative defaults; raise only when real usage demonstrates the need.

**Narrowing requirement.** Every request must provide **at least one** of:

(a) a `filter` containing at least one `data` condition (JSONB containment
    narrows via the GIN), **or**

(b) a block range with `toBlockHeight - fromBlockHeight ≤ N` blocks, where `N`
    is tunable (starting value: 1_000_000).

Requests with only `filter.type` (or nothing) and an unbounded range are
rejected with a clear error.

**Per-branch enforcement.** When `filter.len() > 1`, **each branch** must
independently satisfy the narrowing requirement. Otherwise a single wide
branch dominates the cost regardless of the other branches.

### 4.5 Per-request `statement_timeout`

Inside the read transaction, before running the user query:

```sql
SET LOCAL statement_timeout = '500ms';
```

`SET LOCAL` scopes the value to the current transaction, so it reverts
automatically when the txn commits or rolls back. `paginate_models` already
opens a transaction (`app_ctx.db.begin().await?` in `pagination.rs`), and
the `SET LOCAL` must be issued on that same `DatabaseTransaction` handle —
issuing it outside a transaction would leak the setting to the next request
reusing the pool connection. If a query exceeds the budget despite admission
rules, Postgres cancels it and the resolver returns an error to the client.

This is the **final safeguard** against pathological plans (planner misjudging
JSONB selectivity, hot-contract surprises, stats drift). Nothing else in this
spec can guarantee 500 ms by itself — admission rules keep the request shape
sane, `statement_timeout` keeps its execution bounded.

### 4.6 Tests

- Unit: `parse_filter` rejects `EQUAL` with ≠1 value and `CONTAINS` with 0.
- Unit: admission caps reject oversized filter shapes and unbounded ranges.
- Integration (sqlx test db or testcontainer):
  - Empty filter with a small block range behaves like the current unfiltered
    query restricted to that range.
  - `filter.type` only without a range → rejected.
  - `data` path + `EQUAL`.
  - `data` path + `CONTAINS` (OR across values).
  - Two `Filter`s → OR.
  - Cursor + filter: paging yields the same total set regardless of page size.
  - `fromBlockHeight` / `toBlockHeight` inclusive bounds.
  - Cursor at a position outside the requested range → empty page, not an
    error.
  - Synthetic slow query (wide range + low-selectivity filter crafted to
    bypass admission) → `statement_timeout` fires, error returned to client.

## 5. Performance considerations

The access paths below assume only the two existing indexes — `events-data`
(GIN `jsonb_ops`) and `events-block_height` (btree). No new indexes are
required for v1; §6 describes optional escalations for hot patterns.

**The key limitation to internalize:** the GIN does not preserve order. An
`ORDER BY block_height LIMIT 100` with a JSONB filter must materialize every
matching row and then sort before taking 100. Cost scales with the size of the
match set, not with the `LIMIT`.

| Regime | Plan | Notes |
| --- | --- | --- |
| Highly selective filter (≤ few thousand matches) | Bitmap Index Scan on `events-data`, heap fetch, top-N sort, limit | ms-range. No escalation needed. |
| Medium selectivity (≤ ~100k matches) + narrow block range | `BitmapAnd(events-data, events-block_height)` + sort | tens of ms warm; 100s of ms cold. |
| Low selectivity (millions of matches) | Same plan, sort dominates | **Rejected by §4.4 or cut by §4.5.** |
| No filter, narrow block range | Index scan on `events-block_height` (ordered), cursor-limited | ms; scales with `first`/`last`. |
| No filter, no/broad range | **Rejected by §4.4.** | — |

Statistics tuning, recommended at deploy time:

```sql
ALTER TABLE events ALTER COLUMN data SET STATISTICS 1000;
ANALYZE events;
```

JSONB stats are coarser than scalar stats; raising the target reduces planner
misjudgments on containment selectivity. Re-ANALYZE after large backfills.

## 6. Hot-path index escalation (post-v1, metric-driven)

Do not build specialized indexes up front. The generic GIN + admission rules +
`statement_timeout` handle the general case. Specialized indexes are an
escalation to add **only** when monitoring (§7) shows a specific filter shape
either hitting the timeout ceiling repeatedly or dominating traffic with
visibly strained performance.

### 6.1 Partial btree with expression index

For a pattern like "all `OrderFilled` of a user on the dex, most recent
first":

```sql
CREATE INDEX CONCURRENTLY events_contract_evt_user_bh
  ON events (
    (data->'contract_event'->>'contract'),
    (data->'contract_event'->>'type'),
    (data->'contract_event'->'data'->>'user'),
    block_height DESC, event_idx DESC
  )
  WHERE type = 'contract_event'
    AND (data->'contract_event'->'data'->>'user') IS NOT NULL;
```

Why this is the right shape:

- Equality-filter columns form the prefix in the order callers actually
  filter them.
- `(block_height DESC, event_idx DESC)` sits at the tail, so the index itself
  provides the `ORDER BY`. `LIMIT 100` becomes a bounded walk of 100 index
  entries — O(k), not O(matches).
- The partial `WHERE` excludes unrelated rows (non-`contract_event`, or rows
  without a `user` field), shrinking the index and reducing write amp on
  inserts that will never be targeted by this query.

Caveats:

- Postgres uses this index for queries that filter the **prefix** of the
  indexed columns (contract, or contract + event_type, or all three). Queries
  that skip a middle column (e.g. contract + user, no event_type) rely on
  **skip scan** for multi-column B-tree indexes, which landed in Postgres 18
  (Sep 2025). On older versions the planner typically falls back to the
  generic GIN — acceptable when the remaining filter (e.g. `user`) is
  selective. Event-type cardinality is low (~20 per contract), which makes
  skip scan efficient when available. Confirm the target cluster's major
  version before relying on it.
- Each specialized index is tied to one access pattern. Add only after
  evidence, never preemptively.
- Rows without the extracted field get a `NULL` entry by default in a btree;
  the partial `WHERE … IS NOT NULL` suppresses those entries entirely.
- Expression form (`… ->> 'field'`) extracts as text — fine for the
  stringly-serialized fields we expect in hot paths (`Addr`, event type
  name). For numeric post-filters the expression has to be cast
  (`(… -> 'amount')::numeric`), and the byte-level match the planner uses
  to pick the specialized index becomes correspondingly stricter (§6.3).

### 6.2 Covering post-filters with `INCLUDE`

This is an evolution of §6.1, not an additional index — the same name
(`events_contract_evt_user_bh`) is reused, replacing the earlier definition.

When a non-prefix field is frequently used as an additional filter on top of
the hot-path pattern (e.g. `pair_id` together with the user filter above),
promote it to a covering column:

```sql
CREATE INDEX CONCURRENTLY events_contract_evt_user_bh
  ON events (
    (data->'contract_event'->>'contract'),
    (data->'contract_event'->>'type'),
    (data->'contract_event'->'data'->>'user'),
    block_height DESC, event_idx DESC
  )
  INCLUDE ((data->'contract_event'->'data'->>'pair_id'))
  WHERE type = 'contract_event'
    AND (data->'contract_event'->'data'->>'user') IS NOT NULL;
```

`INCLUDE` stores the expression in the index leaves but does **not** use it
for ordering or search. Postgres can evaluate the post-filter straight from
the index, skipping heap fetches for non-matching rows — dramatic speedup
when the included field is selective.

### 6.3 Resolver routing

When a filter shape matches one of the specialized indexes, the resolver
builds the SQL WHERE using **the same expression form** the index was created
on (not JSONB containment). Postgres's planner picks the specialized index
only when the expression in the query parses to an identical tree as the
index expression; differences in parentheses, casts, or operator forms break
the match.

Design:
- Keep a small, tested set of "shape matchers" in the resolver (one per
  specialized index).
- Shapes that don't match fall through to the generic JSONB containment
  path, which uses the GIN.
- The GraphQL API is unchanged; the caller never knows which index served
  the query.

### 6.4 Generated columns (heavyweight alternative, defer)

Instead of expression indexes, one could add `GENERATED ALWAYS AS (…) STORED`
columns and index them as plain columns. Benefits: columns appear in the
schema, queries are readable, tooling sees them as first-class. Drawback:
adding a `STORED` generated column to a multi-billion-row table **rewrites
the entire table** — an operational event (long lock or `pg_repack`-style
tooling), not a routine migration.

Defer unless there is an independent need for first-class columns
(downstream tooling, joins, `SELECT`-able fields). For a hot-path index the
expression-index form (§6.1/6.2) is equivalent performance with far lower
migration cost.

## 7. Monitoring & statistics

A minimum set of operational hooks to make §6 decisions data-driven rather
than guesswork:

1. **Log slow queries.** Any `events` query whose actual runtime exceeds
   ~200 ms: log the parsed filter shape, block range, cursor position, and
   elapsed time. Periodic review of this log identifies candidates for §6.
2. **Alert on `statement_timeout` hits.** A rising rate of timeouts means
   either abusive traffic (tighten §4.4 caps or add rate limiting upstream)
   or a real access pattern that needs a specialized index.
3. **`EXPLAIN (ANALYZE, BUFFERS)` spot checks.** Before and after adding a
   specialized index, run representative queries and confirm the planner
   picks the intended path. If not, check stats (`ANALYZE events`) and
   expression byte-equality between query and index definition.
4. **Statistics target.** Keep `ALTER TABLE events ALTER COLUMN data SET
   STATISTICS 1000` applied and re-ANALYZE after large ingest catch-ups. The
   default (100) is too coarse for JSONB on a billions-of-rows table.
5. **Admission rejection counter.** Expose a counter for requests rejected
   by §4.4, labeled by rule (shape-cap, no-narrowing, per-branch-unbounded).
   Spikes signal client-side bugs, hostile traffic, or legitimate patterns
   that need a specialized index — distinct from the `statement_timeout`
   signal above.

## 8. Non-goals

- No changes to the `events` subscription public GraphQL schema: the shared
  filter module is an internal refactor, and the subscription keeps the
  current `Filter` / `FilterData` / `CheckValue` wire names via
  `#[graphql(name = "...")]` aliases (see §4.1).
- No denormalized per-event-type tables (e.g. `mailbox_deliveries`). That
  option becomes attractive only at ≳ 10⁸ matching rows per pattern and
  remains a future Dango-indexer-side change.
- No filtering by tx hash or tx sender on this query — those would require
  joining `transactions` and are out of scope.
- No ordering other than by `(block_height, event_idx)`.
- No preemptive creation of specialized indexes. §6 is explicitly metric-driven.
- No bypass of `statement_timeout` for "trusted" callers: every request
  carries the same budget.

## 9. Open questions

1. Concrete values for the admission caps in §4.4 and for the block-range
   ceiling `N`: the table lists conservative defaults; tune in the first
   weeks of production traffic.
2. Should the resolver expose alias shortcuts (e.g. `contractAddr`,
   `contractEventType`) for the common `contract_event.*` paths? Pure
   ergonomics, strictly additive — can land later without breaking the
   generic API.
3. Which production Postgres version will this ship against? Multi-column
   btree skip scan landed in Postgres 18 (Sep 2025); on ≤ 17 the planner
   falls back to the generic GIN for queries that skip a middle column of a
   specialized index (§6.1). The monitoring playbook (§7.3) should be
   adjusted accordingly.
