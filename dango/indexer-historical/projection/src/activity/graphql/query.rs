//! The activity projection's GraphQL **read surface** — the resolvers that run
//! the eight documented feeds (see `DESIGN.md` § Access paths).
//!
//! Eight feeds collapse into five fields by folding each "+ optional filter"
//! pair into one resolver argument:
//!
//! | field | queries | filters |
//! |-------|---------|---------|
//! | `transactionsInvolving` | 1 | address (+ optional `role`, `kind`) |
//! | `eventsByType` | 2 | type |
//! | `contractEvents` | 3 / 4 | contract (+ optional `names`) |
//! | `eventsInvolving` | 5 / 6 | address (+ optional `type`) |
//! | `contractEventsInvolving` | 7 / 8 | address + contract (+ optional `names`) |
//!
//! Each is newest-first, capped at [`MAX_LIMIT`](super::pagination::MAX_LIMIT),
//! and keyset-paginated via a forward Relay connection. The SQL is hand-written
//! to match the access paths exactly (`DISTINCT ON` to collapse an event's
//! participant rows, a row-comparison keyset, an in-index `IN (…)` name list,
//! the union of involved ∪ sender for query 1) and parameterized through
//! [`Binder`] so no argument is ever interpolated raw. It references the tables
//! by their real names — `activity_transactions`, `activity_events`,
//! `activity_event_data` (the `activity_` prefix the migrations and entities
//! create) — never the bare `events` / `transactions`: a short name compiles
//! fine but fails at runtime against Postgres (`relation "events" does not
//! exist`). The `feeds_execute_against_postgres` integration test in
//! [`super::super`] runs every feed against a real Postgres so that, and the
//! `UNION` parenthesisation, can never regress unnoticed. The shared read
//! handles come from the schema context (`ctx.data`), injected when the schema
//! is built — so this module depends only on `async-graphql`, `sea-orm`, and
//! the projection's own entities, never on the httpd.

use {
    super::{
        pagination::{Binder, EventCursor, UnitCursor, decode_after, page_limit, paginate},
        types::{Address, AddressRole, Event, EventRow, Hash, Transaction, UnitKind},
    },
    crate::{
        activity::{entity::transactions, event_type::EventType},
        metrics::timed_query,
    },
    async_graphql::{
        Context, Object, Result,
        connection::{Connection, OpaqueCursor},
    },
    sea_orm::{
        ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, FromQueryResult, QueryFilter,
        QueryOrder, Statement,
    },
};

/// The event-position ordering, newest-first — shared by every event feed so
/// the index serves it as a backward scan (no sort). The `DISTINCT ON` feeds
/// append `, address DESC` (the tiebreaker that picks each event's
/// representative row): it must be **DESC** too, so the whole `ORDER BY` is the
/// backward scan verbatim — `address ASC` would force an Incremental Sort
/// (verified via `EXPLAIN`; the address-led feeds need no tiebreaker, so they
/// stop here).
const POS_DESC: &str = "block_height DESC, category DESC, category_index DESC, event_index DESC";

/// Read surface of the activity projection.
#[derive(Default)]
pub struct ActivityQuery;

#[Object]
impl ActivityQuery {
    /// Every transaction with a given content hash, newest-first — the
    /// un-paginated lookup behind the detail view. The hash is **not unique**
    /// (see `DESIGN.md` § Identity): identical tx bytes can be re-included in a
    /// later block (a failed tx doesn't consume its nonce; a contract sender has
    /// none), so one hash may map to several units. Returns every match ordered
    /// by position DESC (each row's on-demand `tx` / `outcome` fields hydrate the
    /// full payload from its block); empty if none. Cron units carry no hash, so
    /// this only ever resolves transactions. Served by the partial `(hash)`
    /// index.
    ///
    /// **Intentionally un-capped.** A hash collision is only reachable by
    /// re-submitting *byte-identical* tx bytes — a nonce-less contract sender, or
    /// a tx that passes `CheckTx` but fails in `FinalizeBlock` — never by an
    /// external party crafting distinct txs, so in practice a hash maps to a
    /// handful of units and the un-paginated list (plus its per-row `tx` /
    /// `outcome` block hydration) is bounded in practice. We accept the unbounded
    /// shape rather than complicate the detail lookup. If a deployment ever
    /// observes *many* units sharing one hash, add a `LIMIT` here — the partial
    /// `(hash)` index already serves it as a bounded backward scan.
    async fn transactions_by_hash(
        &self,
        ctx: &Context<'_>,
        hash: Hash,
    ) -> Result<Vec<Transaction>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let rows = timed_query(
            "transactions_by_hash",
            transactions::Entity::find()
                .filter(transactions::Column::Hash.eq(hash.bytes()))
                .order_by_desc(transactions::Column::BlockHeight)
                .order_by_desc(transactions::Column::Idx)
                .all(db),
        )
        .await?;
        rows.into_iter().map(Transaction::try_from).collect()
    }

    /// Query 1 — transactions (and cronjobs) **involving** an address: by
    /// default the unit's `sender` *or* a party to one of the unit's events.
    /// The two sides live in different tables (`transactions.sender` and the
    /// `events` participation rows); they are unioned, deduped, and the newest N
    /// kept. `role` narrows to one side (`SENDER` / `PARTICIPANT`); omitted ⇒
    /// either. Cron units (which have no sender) are included by default; `kind`
    /// narrows to transactions or cronjobs only.
    async fn transactions_involving(
        &self,
        ctx: &Context<'_>,
        address: Address,
        role: Option<AddressRole>,
        kind: Option<UnitKind>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<UnitCursor>, Transaction>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<UnitCursor>(after)?;
        let fetch = limit + 1;

        // Which sides of the union to scan. `role` picks one; cron has no
        // sender, so a cron restriction drops the sender side regardless.
        let want_sender = !matches!(role, Some(AddressRole::Participant))
            && !matches!(kind, Some(UnitKind::Cron));
        let want_participant = !matches!(role, Some(AddressRole::Sender));

        let mut binder = Binder::new();
        let address_ph = binder.bind(address.bytes());
        // The unit keyset is bound once and reused by whichever sides are scanned.
        let keyset = after.map(|c| {
            (
                binder.bind(c.block_height),
                binder.bind(c.kind),
                binder.bind(c.idx),
            )
        });

        let mut sides: Vec<String> = Vec::new();

        // Involved side: the distinct units X is a party to, from `events`.
        if want_participant {
            let kind_filter = match kind {
                Some(k) => format!(" AND category = {}", binder.bind(k.code())),
                None => String::new(),
            };
            let keyset_clause = match &keyset {
                Some((h, k, i)) => {
                    format!(" AND (block_height, category, category_index) < ({h}, {k}, {i})")
                },
                None => String::new(),
            };
            sides.push(format!(
                "SELECT DISTINCT block_height, category, category_index FROM activity_events \
                 WHERE address = {address_ph}{kind_filter}{keyset_clause} \
                 ORDER BY block_height DESC, category DESC, category_index DESC LIMIT {fetch}"
            ));
        }

        // Sender side: the units X sent, from `transactions`. Only transactions
        // carry a sender (cron rows are NULL), so every row here is `kind = Tx`
        // — a constant. The `ORDER BY` therefore drops `kind` and matches the
        // `(sender, block_height, idx)` index verbatim (a clean backward scan):
        // keeping `kind` would wedge a column the index lacks into the sort and
        // force an Incremental Sort (`EXPLAIN`-verified). The keyset still
        // compares the full unit position so it stays consistent with the
        // cron-bearing involved side and the outer ordering.
        if want_sender {
            let keyset_clause = match &keyset {
                Some((h, k, i)) => format!(" AND (block_height, kind, idx) < ({h}, {k}, {i})"),
                None => String::new(),
            };
            sides.push(format!(
                "SELECT block_height, kind AS category, idx AS category_index FROM activity_transactions \
                 WHERE sender = {address_ph}{keyset_clause} \
                 ORDER BY block_height DESC, idx DESC LIMIT {fetch}"
            ));
        }

        // No side matches this (role, kind) combo — e.g. `SENDER` on cron-only.
        // Nothing can match, so skip the database and return an empty page.
        let rows = if sides.is_empty() {
            Vec::new()
        } else {
            // Merge (dedup is the UNION), join the unit rows, take the newest N.
            // Each arm carries its own `ORDER BY` / `LIMIT`, so it MUST be
            // parenthesised — Postgres rejects `SELECT … LIMIT n UNION …`
            // without parens (syntax error). A lone arm is just `(SELECT …)`.
            let units = sides
                .iter()
                .map(|side| format!("({side})"))
                .collect::<Vec<_>>()
                .join(" UNION ");
            let sql = format!(
                "WITH unit AS ({units}) \
                 SELECT t.* FROM unit u JOIN activity_transactions t \
                 ON t.block_height = u.block_height AND t.kind = u.category AND t.idx = u.category_index \
                 ORDER BY t.block_height DESC, t.kind DESC, t.idx DESC LIMIT {fetch}"
            );
            let stmt =
                Statement::from_sql_and_values(DbBackend::Postgres, sql, binder.into_values());
            timed_query(
                "transactions_involving",
                transactions::Entity::find().from_raw_sql(stmt).all(db),
            )
            .await?
        };

        paginate(
            rows,
            limit,
            after.is_some(),
            |m| UnitCursor {
                block_height: m.block_height,
                kind: m.kind,
                idx: m.idx,
            },
            Transaction::try_from,
        )
    }

    /// Query 2 — events of a given type, newest-first.
    async fn events_by_type(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "type")] ty: EventType,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<EventCursor>(after)?;
        let fetch = limit + 1;

        let mut binder = Binder::new();
        let type_ph = binder.bind(ty.code());
        let keyset = event_keyset(&mut binder, &after);
        let sql = format!(
            "SELECT DISTINCT ON (block_height, category, category_index, event_index) * \
             FROM activity_events WHERE event_type = {type_ph}{keyset} \
             ORDER BY {POS_DESC}, address DESC LIMIT {fetch}"
        );
        run_event_feed(
            db,
            "events_by_type",
            sql,
            binder,
            limit,
            after.is_some(),
            wants_event_data(ctx),
        )
        .await
    }

    /// Queries 3 / 4 — contract events emitted by a contract, optionally
    /// narrowed to a set of event names (a single value or a list).
    async fn contract_events(
        &self,
        ctx: &Context<'_>,
        contract: Address,
        names: Option<Vec<String>>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<EventCursor>(after)?;
        let fetch = limit + 1;

        let mut binder = Binder::new();
        let contract_ph = binder.bind(contract.bytes());
        let names = name_filter(&mut binder, &names);
        let keyset = event_keyset(&mut binder, &after);
        let sql = format!(
            "SELECT DISTINCT ON (block_height, category, category_index, event_index) * \
             FROM activity_events WHERE contract = {contract_ph}{names}{keyset} \
             ORDER BY {POS_DESC}, address DESC LIMIT {fetch}"
        );
        run_event_feed(
            db,
            "contract_events",
            sql,
            binder,
            limit,
            after.is_some(),
            wants_event_data(ctx),
        )
        .await
    }

    /// Queries 5 / 6 — events **involving** an address, optionally of a given
    /// type. One row per event (X appears once per event), so no `DISTINCT`.
    async fn events_involving(
        &self,
        ctx: &Context<'_>,
        address: Address,
        #[graphql(name = "type")] ty: Option<EventType>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<EventCursor>(after)?;
        let fetch = limit + 1;

        let mut binder = Binder::new();
        let address_ph = binder.bind(address.bytes());
        let type_filter = match ty {
            Some(t) => format!(" AND event_type = {}", binder.bind(t.code())),
            None => String::new(),
        };
        let keyset = event_keyset(&mut binder, &after);
        let sql = format!(
            "SELECT * FROM activity_events WHERE address = {address_ph}{type_filter}{keyset} \
             ORDER BY {POS_DESC} LIMIT {fetch}"
        );
        run_event_feed(
            db,
            "events_involving",
            sql,
            binder,
            limit,
            after.is_some(),
            wants_event_data(ctx),
        )
        .await
    }

    /// Queries 7 / 8 — contract events of a contract **involving** an address,
    /// optionally narrowed to a set of event names.
    async fn contract_events_involving(
        &self,
        ctx: &Context<'_>,
        address: Address,
        contract: Address,
        names: Option<Vec<String>>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<EventCursor>(after)?;
        let fetch = limit + 1;

        let mut binder = Binder::new();
        let address_ph = binder.bind(address.bytes());
        let contract_ph = binder.bind(contract.bytes());
        let names = name_filter(&mut binder, &names);
        let keyset = event_keyset(&mut binder, &after);
        let sql = format!(
            "SELECT * FROM activity_events \
             WHERE address = {address_ph} AND contract = {contract_ph}{names}{keyset} \
             ORDER BY {POS_DESC} LIMIT {fetch}"
        );
        run_event_feed(
            db,
            "contract_events_involving",
            sql,
            binder,
            limit,
            after.is_some(),
            wants_event_data(ctx),
        )
        .await
    }
}

// ---- shared resolver plumbing ----

/// Wrap a feed's event query so each row also carries its stored payload: the
/// inner query (already selected, ordered, and limited over `events`) becomes a
/// subquery, and `data` is pulled per row by a **correlated** lookup on
/// `event_data`'s primary key. A scalar subquery (not a join) guarantees a
/// point index probe per row no matter how large `event_data` grows — a plain
/// `LEFT JOIN` lets the planner pick a hash join that sequentially scans the
/// whole `event_data` (verified: it does so on small tables). The inner `LIMIT`
/// blocks subquery flattening, so the inner's backward-scan (no-sort) plan is
/// left untouched; `data` is `NULL` for non-priority events not stored there.
///
/// When the client did **not** select the `data` field (`wants_data` false), the
/// per-row probe is skipped and the column is bound to `NULL` — the feed keeps
/// its shape ([`EventRow`] still maps `data`) at zero payload cost. (Even a
/// missed detection is safe: [`Event::data`] would just hydrate from the block.)
fn with_event_data(inner: &str, wants_data: bool) -> String {
    let data_expr = if wants_data {
        "( SELECT ed.data FROM activity_event_data ed \
          WHERE ed.block_height = sub.block_height AND ed.category = sub.category \
          AND ed.category_index = sub.category_index AND ed.event_index = sub.event_index )"
    } else {
        "NULL::bytea"
    };
    format!(
        "SELECT sub.*, {data_expr} AS data \
         FROM ({inner}) sub \
         ORDER BY sub.block_height DESC, sub.category DESC, sub.category_index DESC, \
         sub.event_index DESC"
    )
}

/// Whether the client selected the event `data` field under this feed — through
/// either Relay shape (`edges { node { data } }` or `nodes { data }`). Drives
/// [`with_event_data`]'s eager payload probe: skip it when `data` is absent.
/// Conservative by construction — an undetected selection only costs a later
/// per-row block hydration in [`Event::data`], never correctness.
fn wants_event_data(ctx: &Context<'_>) -> bool {
    let look_ahead = ctx.look_ahead();
    look_ahead
        .field("edges")
        .field("node")
        .field("data")
        .exists()
        || look_ahead.field("nodes").field("data").exists()
}

/// Run an event feed's statement and shape the rows into a connection. Every
/// feed's inner query is wrapped by [`with_event_data`] and mapped to an
/// [`EventRow`] (the event columns plus the correlated payload), so all share
/// one cursor / node mapping.
async fn run_event_feed(
    db: &DatabaseConnection,
    query: &'static str,
    sql: String,
    binder: Binder,
    limit: u64,
    has_prev: bool,
    wants_data: bool,
) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        with_event_data(&sql, wants_data),
        binder.into_values(),
    );
    let rows = timed_query(query, EventRow::find_by_statement(stmt).all(db)).await?;
    paginate(
        rows,
        limit,
        has_prev,
        |m| EventCursor {
            block_height: m.block_height,
            category: m.category,
            category_index: m.category_index,
            event_index: m.event_index,
        },
        Event::try_from,
    )
}

/// The keyset predicate for an event feed: `AND (position) < ($cursor)` (a
/// row-comparison the index serves directly), or empty for the first page.
fn event_keyset(binder: &mut Binder, after: &Option<EventCursor>) -> String {
    match after {
        Some(c) => format!(
            " AND (block_height, category, category_index, event_index) < ({}, {}, {}, {})",
            binder.bind(c.block_height),
            binder.bind(c.category),
            binder.bind(c.category_index),
            binder.bind(c.event_index),
        ),
        None => String::new(),
    }
}

/// The optional contract-event-name filter: `AND contract_event_name IN (…)`
/// bound in-index, or empty when no (non-empty) name list was given.
fn name_filter(binder: &mut Binder, names: &Option<Vec<String>>) -> String {
    match names {
        Some(list) if !list.is_empty() => {
            let placeholders = list
                .iter()
                .map(|name| binder.bind(name.clone()))
                .collect::<Vec<_>>()
                .join(", ");
            format!(" AND contract_event_name IN ({placeholders})")
        },
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        async_graphql::{EmptyMutation, EmptySubscription, Schema},
    };

    /// The read surface is well-formed and pins the choices that are easy to
    /// regress: the blockchain scalars, the `type` argument rename, the name
    /// list filter, and `timestamp` as a string (so its nanosecond value never
    /// loses precision in a 64-bit-lossy client).
    #[test]
    fn read_surface_is_well_formed() {
        let sdl = Schema::build(ActivityQuery, EmptyMutation, EmptySubscription)
            .finish()
            .sdl();

        // Custom scalars and the camelCased feeds.
        for needle in [
            "scalar Address",
            "scalar Hash",
            "scalar Tx",
            // The hash lookup returns a list — the hash is not unique.
            "transactionsByHash(hash: Hash!): [Transaction!]!",
            "transactionsInvolving(",
            "eventsByType(",
            "contractEventsInvolving(",
        ] {
            assert!(sdl.contains(needle), "missing `{needle}`:\n{sdl}");
        }

        // The `type` keyword is exposed as an argument, the name filter is a
        // list, and the cron/tx and sender/participant axes are enums.
        assert!(sdl.contains("type: EventType!"), "type arg:\n{sdl}");
        assert!(sdl.contains("names: [String!]"), "names list arg:\n{sdl}");
        assert!(sdl.contains("kind: UnitKind"), "kind arg:\n{sdl}");
        assert!(sdl.contains("role: AddressRole"), "role arg:\n{sdl}");

        // `timestamp` is a string, never a (lossy) Int.
        assert!(sdl.contains("timestamp: String!"), "timestamp type:\n{sdl}");

        // The on-demand detail fields: the full tx as the native `Tx` scalar,
        // the outcome as a well-formed `JSON` scalar (not a dangling type ref).
        assert!(sdl.contains("tx: Tx"), "tx detail field:\n{sdl}");
        assert!(
            sdl.contains("outcome: JSON"),
            "outcome detail field:\n{sdl}"
        );

        // The event payload is hydrated on demand as a `JSON` scalar too.
        assert!(sdl.contains("data: JSON"), "event data field:\n{sdl}");
    }
}
