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
//! [`Binder`] so no argument is ever interpolated raw. The shared read handles
//! come from the schema context (`ctx.data`), injected when the schema is built
//! — so this module depends only on `async-graphql`, `sea-orm`, and the
//! projection's own entities, never on the httpd.

use {
    super::{
        pagination::{Binder, EventCursor, UnitCursor, decode_after, page_limit, paginate},
        types::{Address, AddressRole, Event, EventRow, Hash, Transaction, UnitKind},
    },
    crate::activity::{entity::transactions, event_type::EventType},
    async_graphql::{
        Context, Object, Result,
        connection::{Connection, OpaqueCursor},
    },
    sea_orm::{
        ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, FromQueryResult, QueryFilter,
        Statement,
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
    /// A single transaction by its content hash — the un-paginated point lookup
    /// behind the detail view. Returns the indexed summary row (whose on-demand
    /// `tx` / `outcome` fields hydrate the full payload from the block); `None`
    /// if no transaction has that hash. Cron units carry no hash, so this only
    /// ever resolves transactions. Served by the partial `(hash)` index.
    async fn transaction(&self, ctx: &Context<'_>, hash: Hash) -> Result<Option<Transaction>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let row = transactions::Entity::find()
            .filter(transactions::Column::Hash.eq(hash.bytes()))
            .one(db)
            .await?;
        row.map(Transaction::try_from).transpose()
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
                "SELECT DISTINCT block_height, category, category_index FROM events \
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
                "SELECT block_height, kind AS category, idx AS category_index FROM transactions \
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
                 SELECT t.* FROM unit u JOIN transactions t \
                 ON t.block_height = u.block_height AND t.kind = u.category AND t.idx = u.category_index \
                 ORDER BY t.block_height DESC, t.kind DESC, t.idx DESC LIMIT {fetch}"
            );
            let stmt =
                Statement::from_sql_and_values(DbBackend::Postgres, sql, binder.into_values());
            transactions::Entity::find()
                .from_raw_sql(stmt)
                .all(db)
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
             FROM events WHERE event_type = {type_ph}{keyset} \
             ORDER BY {POS_DESC}, address DESC LIMIT {fetch}"
        );
        run_event_feed(db, sql, binder, limit, after.is_some()).await
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
             FROM events WHERE contract = {contract_ph}{names}{keyset} \
             ORDER BY {POS_DESC}, address DESC LIMIT {fetch}"
        );
        run_event_feed(db, sql, binder, limit, after.is_some()).await
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
            "SELECT * FROM events WHERE address = {address_ph}{type_filter}{keyset} \
             ORDER BY {POS_DESC} LIMIT {fetch}"
        );
        run_event_feed(db, sql, binder, limit, after.is_some()).await
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
            "SELECT * FROM events \
             WHERE address = {address_ph} AND contract = {contract_ph}{names}{keyset} \
             ORDER BY {POS_DESC} LIMIT {fetch}"
        );
        run_event_feed(db, sql, binder, limit, after.is_some()).await
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
fn with_event_data(inner: &str) -> String {
    format!(
        "SELECT sub.*, ( \
         SELECT ed.data FROM event_data ed \
         WHERE ed.block_height = sub.block_height AND ed.category = sub.category \
         AND ed.category_index = sub.category_index AND ed.event_index = sub.event_index \
         ) AS data \
         FROM ({inner}) sub \
         ORDER BY sub.block_height DESC, sub.category DESC, sub.category_index DESC, \
         sub.event_index DESC"
    )
}

/// Run an event feed's statement and shape the rows into a connection. Every
/// feed's inner query is wrapped by [`with_event_data`] and mapped to an
/// [`EventRow`] (the event columns plus the correlated payload), so all share
/// one cursor / node mapping.
async fn run_event_feed(
    db: &DatabaseConnection,
    sql: String,
    binder: Binder,
    limit: u64,
    has_prev: bool,
) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        with_event_data(&sql),
        binder.into_values(),
    );
    let rows = EventRow::find_by_statement(stmt).all(db).await?;
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
            "transaction(hash: Hash!)",
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
