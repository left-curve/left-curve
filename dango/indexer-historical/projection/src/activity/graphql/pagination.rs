//! Keyset pagination for the activity feeds — the small, shared engine the
//! resolvers in [`super::query`] run on.
//!
//! Every feed is newest-first and paginated on its **ordering tuple** (the
//! event position, or the unit position for query 1), never `OFFSET`: the
//! cursor is that tuple, and "the next page" is `WHERE (tuple) < $cursor`. The
//! cursor is opaque to clients ([`OpaqueCursor`] = base64 of the serialized
//! tuple), so the wire form never leaks the layout.
//!
//! [`Binder`] keeps the hand-written SQL and its bound values in lockstep, and
//! [`paginate`] turns the `limit + 1` fetched rows into a Relay
//! [`Connection`] — the extra row is how `hasNextPage` is known without a
//! `COUNT`.

use {
    async_graphql::{
        Error, OutputType, Result,
        connection::{Connection, CursorType, Edge, EmptyFields, OpaqueCursor},
    },
    sea_orm::Value,
    serde::{Deserialize, Serialize, de::DeserializeOwned},
};

/// Page size when a feed is queried without an explicit `first`.
pub(crate) const DEFAULT_LIMIT: u64 = 50;
/// Hard ceiling on `first` — a page never returns more than this many rows.
pub(crate) const MAX_LIMIT: u64 = 200;

/// The effective page size for a `first` argument, clamped to `[1, MAX_LIMIT]`
/// (and defaulted when absent or non-positive).
pub(crate) fn page_limit(first: Option<i32>) -> u64 {
    match first {
        Some(n) if n > 0 => (n as u64).min(MAX_LIMIT),
        _ => DEFAULT_LIMIT,
    }
}

// ---- cursors ----

/// The keyset of an event feed: the event position, compared all-DESC.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct EventCursor {
    pub block_height: i64,
    pub category: i16,
    pub category_index: i32,
    pub event_index: i32,
}

/// The keyset of query 1 (transactions involving X): the unit position.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct UnitCursor {
    pub block_height: i64,
    pub kind: i16,
    pub idx: i32,
}

/// Decode an opaque `after` cursor into its keyset, or `None` for the first
/// page. A malformed cursor is a user error, surfaced as such.
pub(crate) fn decode_after<C>(after: Option<String>) -> Result<Option<C>>
where
    C: Serialize + DeserializeOwned + Send + Sync,
{
    after
        .map(|raw| {
            OpaqueCursor::<C>::decode_cursor(&raw)
                .map(|cursor| cursor.0)
                .map_err(|err| Error::new(format!("invalid cursor: {err}")))
        })
        .transpose()
}

// ---- parameter binding ----

/// Accumulates the bound values of a hand-written, parameterized statement,
/// returning the matching `$n` placeholder for each value as it is bound — so
/// the SQL text and its value list can never drift out of order, and no
/// caller-supplied value is ever interpolated into the SQL raw.
#[derive(Default)]
pub(crate) struct Binder {
    values: Vec<Value>,
}

impl Binder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Bind one value, returning its `$n` placeholder.
    pub(crate) fn bind<V>(&mut self, value: V) -> String
    where
        V: Into<Value>,
    {
        self.values.push(value.into());
        format!("${}", self.values.len())
    }

    pub(crate) fn into_values(self) -> Vec<Value> {
        self.values
    }
}

// ---- connection assembly ----

/// Turn up to `limit + 1` rows (newest-first) into a forward Relay
/// [`Connection`]: the surplus row, if any, sets `hasNextPage` and is dropped;
/// each kept row becomes an edge tagged with its keyset cursor. `has_prev` is
/// whether the caller paginated past the first page.
pub(crate) fn paginate<M, C, N, Cur, Node>(
    mut rows: Vec<M>,
    limit: u64,
    has_prev: bool,
    cursor_of: Cur,
    node_of: Node,
) -> Result<Connection<OpaqueCursor<C>, N, EmptyFields, EmptyFields>>
where
    C: Serialize + DeserializeOwned + Send + Sync,
    N: OutputType,
    Cur: Fn(&M) -> C,
    Node: Fn(M) -> Result<N>,
{
    let has_next = rows.len() as u64 > limit;
    rows.truncate(limit as usize);

    let mut connection = Connection::new(has_prev, has_next);
    for row in rows {
        let cursor = OpaqueCursor(cursor_of(&row));
        let node = node_of(row)?;
        connection.edges.push(Edge::new(cursor, node));
    }

    Ok(connection)
}
