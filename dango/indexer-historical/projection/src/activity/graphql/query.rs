//! The activity projection's GraphQL **read surface** — the resolvers that run
//! the eight documented feeds (see `DESIGN.md` § Access paths).
//!
//! Eight feeds collapse into five fields by folding each "+ optional filter"
//! pair into one resolver argument:
//!
//! | field | queries | filters |
//! |-------|---------|---------|
//! | `transactionsInvolving` | 1 | address (+ optional `kind`) |
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
        types::{Address, Event, Transaction, UnitKind},
    },
    crate::activity::{
        entity::{events, transactions},
        event_type::EventType,
    },
    async_graphql::{
        Context, Object, Result,
        connection::{Connection, OpaqueCursor},
    },
    sea_orm::{DatabaseConnection, DbBackend, EntityTrait, Statement},
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
    /// Query 1 — transactions (and cronjobs) **involving** an address: it is the
    /// unit's `sender` *or* a party to one of the unit's events. The two sides
    /// live in different tables (`transactions.sender` and the `events`
    /// participation rows); they are unioned, deduped, and the newest N kept.
    /// Cron units (which have no sender) are included by default; `kind` narrows
    /// to transactions or cronjobs only.
    async fn transactions_involving(
        &self,
        ctx: &Context<'_>,
        address: Address,
        kind: Option<UnitKind>,
        first: Option<i32>,
        after: Option<String>,
    ) -> Result<Connection<OpaqueCursor<UnitCursor>, Transaction>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let limit = page_limit(first);
        let after = decode_after::<UnitCursor>(after)?;
        let fetch = limit + 1;

        let mut binder = Binder::new();
        let address_ph = binder.bind(address.bytes());
        // The unit keyset is bound once and reused on both sides of the union.
        let keyset = after.map(|c| {
            (
                binder.bind(c.block_height),
                binder.bind(c.kind),
                binder.bind(c.idx),
            )
        });

        // Involved side: the distinct units X is a party to, from `events`.
        let involved_kind = match kind {
            Some(k) => format!(" AND category = {}", binder.bind(k.code())),
            None => String::new(),
        };
        let involved_keyset = match &keyset {
            Some((h, k, i)) => {
                format!(" AND (block_height, category, category_index) < ({h}, {k}, {i})")
            },
            None => String::new(),
        };
        let involved = format!(
            "SELECT DISTINCT block_height, category, category_index FROM events \
             WHERE address = {address_ph}{involved_kind}{involved_keyset} \
             ORDER BY block_height DESC, category DESC, category_index DESC LIMIT {fetch}"
        );

        // Sender side: the units X sent, from `transactions`. Cron has no
        // sender, so when the caller restricts to cron it is omitted entirely.
        let units = if matches!(kind, Some(UnitKind::Cron)) {
            involved
        } else {
            let sender_keyset = match &keyset {
                Some((h, k, i)) => format!(" AND (block_height, kind, idx) < ({h}, {k}, {i})"),
                None => String::new(),
            };
            let sender = format!(
                "SELECT block_height, kind AS category, idx AS category_index FROM transactions \
                 WHERE sender = {address_ph}{sender_keyset} \
                 ORDER BY block_height DESC, kind DESC, idx DESC LIMIT {fetch}"
            );
            format!("{involved} UNION {sender}")
        };

        // Merge (dedup is the UNION), join the unit rows, take the newest N.
        let sql = format!(
            "WITH unit AS ({units}) \
             SELECT t.* FROM unit u JOIN transactions t \
             ON t.block_height = u.block_height AND t.kind = u.category AND t.idx = u.category_index \
             ORDER BY t.block_height DESC, t.kind DESC, t.idx DESC LIMIT {fetch}"
        );

        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, binder.into_values());
        let rows = transactions::Entity::find()
            .from_raw_sql(stmt)
            .all(db)
            .await?;
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

/// Run an event feed's statement and shape the rows into a connection. Every
/// event feed selects the full `events` row, so all map straight to
/// [`events::Model`] and share one cursor / node mapping.
async fn run_event_feed(
    db: &DatabaseConnection,
    sql: String,
    binder: Binder,
    limit: u64,
    has_prev: bool,
) -> Result<Connection<OpaqueCursor<EventCursor>, Event>> {
    let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, binder.into_values());
    let rows = events::Entity::find().from_raw_sql(stmt).all(db).await?;
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
            "transactionsInvolving(",
            "eventsByType(",
            "contractEventsInvolving(",
        ] {
            assert!(sdl.contains(needle), "missing `{needle}`:\n{sdl}");
        }

        // The `type` keyword is exposed as an argument, the name filter is a
        // list, and the cron/tx axis is an enum.
        assert!(sdl.contains("type: EventType!"), "type arg:\n{sdl}");
        assert!(sdl.contains("names: [String!]"), "names list arg:\n{sdl}");
        assert!(sdl.contains("kind: UnitKind"), "kind arg:\n{sdl}");

        // `timestamp` is a string, never a (lossy) Int.
        assert!(sdl.contains("timestamp: String!"), "timestamp type:\n{sdl}");
    }
}
