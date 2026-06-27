//! The activity projection's GraphQL **type surface**: the blockchain-primitive
//! scalars (`Address`, `Hash`) and the output objects (`Transaction`, `Event`)
//! the read resolvers return. The feed resolvers live in [`super::query`], the
//! keyset machinery in [`super::pagination`]; the exception is `Transaction`'s
//! on-demand `tx` / `outcome` detail fields, hydrated from the unit's block via
//! the shared [`BlockLoader`] `DataLoader` and resolved here next to the type.
//!
//! Addresses and hashes are stored as raw bytes (`BYTEA`) but exposed as their
//! canonical text form via thin newtypes over grug's `Addr` / `Hash256` — so
//! the API speaks the same hex dialect as the rest of dango: an `Address` is
//! lowercase `0x`-prefixed, a `Hash` is bare uppercase. The newtypes exist
//! because the scalar trait and those primitives are both foreign here.

use {
    crate::activity::{
        decompress_event, entity::transactions, event_type::EventType, flatten_unit,
    },
    async_graphql::{
        ComplexObject, Context, Enum, Error, InputValueError, InputValueResult, Json, Result,
        Scalar, ScalarType, SimpleObject, Value, dataloader::DataLoader,
    },
    dango_indexer_historical_block_source::BlockLoader,
    dango_indexer_historical_types::BlockData,
    dango_primitives::{
        Addr, CronOutcome, FlatCategory, FlatEvent, Hash256, Timestamp, Tx, TxOutcome,
    },
    sea_orm::FromQueryResult,
    serde::Serialize,
    std::{str::FromStr, sync::Arc},
};

// ---- scalars ----

/// A 20-byte account or contract address, serialized as lowercase
/// `0x`-prefixed hex (grug's `Addr` convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address(pub Addr);

#[Scalar]
impl ScalarType for Address {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Addr::from_str(&s)
                .map(Address)
                .map_err(InputValueError::custom),
            other => Err(InputValueError::expected_type(other)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

impl Address {
    /// The raw 20 bytes, as stored in the `address` / `contract` columns.
    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }
}

/// A 32-byte content hash, serialized as bare uppercase hex (grug's `Hash256`
/// convention — no `0x` prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hash(pub Hash256);

#[Scalar]
impl ScalarType for Hash {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => Hash256::from_str(&s)
                .map(Hash)
                .map_err(InputValueError::custom),
            other => Err(InputValueError::expected_type(other)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

impl Hash {
    /// The raw 32 bytes, as stored in the `hash` column.
    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }
}

/// `Addr` from stored bytes — a stored address is always 20 bytes, so a
/// wrong-length value is a data-integrity error surfaced to the caller.
fn address_from_bytes(bytes: Vec<u8>) -> Result<Address, Error> {
    Addr::try_from(bytes)
        .map(Address)
        .map_err(|err| Error::new(format!("invalid stored address: {err}")))
}

/// `Hash256` from stored bytes — same contract as [`address_from_bytes`].
fn hash_from_bytes(bytes: Vec<u8>) -> Result<Hash, Error> {
    Hash256::try_from(bytes)
        .map(Hash)
        .map_err(|err| Error::new(format!("invalid stored hash: {err}")))
}

// ---- enums ----

/// How an `address` argument must relate to a unit in the
/// `transactionsInvolving` feed. Omitted ⇒ **either** — the address sent the
/// unit *or* was a party to one of its events (their union, the broad
/// "involving" sense). `sender` and `participant` are genuinely different sets:
/// the sender is recorded in `transactions.sender`, not auto-added as an event
/// participant (see `DESIGN.md` § Tables).
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressRole {
    /// The address sent the unit (`transactions.sender`). Cron units have no
    /// sender, so this never matches a cronjob.
    Sender,
    /// The address is a party to one of the unit's events (the participation
    /// rows in `events`).
    Participant,
}

/// The kind of an executed unit — mirrors [`dango_primitives::FlatCategory`].
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Cron,
    Transaction,
}

impl UnitKind {
    /// The stored discriminant for this kind (the value in `transactions.kind`
    /// / `events.category`).
    pub(crate) fn code(self) -> i16 {
        match self {
            Self::Cron => FlatCategory::Cron as i16,
            Self::Transaction => FlatCategory::Tx as i16,
        }
    }

    fn from_code(code: i16) -> Result<Self, Error> {
        match code {
            c if c == FlatCategory::Cron as i16 => Ok(Self::Cron),
            c if c == FlatCategory::Tx as i16 => Ok(Self::Transaction),
            other => Err(Error::new(format!("unknown unit kind: {other}"))),
        }
    }
}

// ---- output objects ----

/// One event feed row: the event's stored columns plus its payload blob from
/// the `event_data` join — `None` for non-priority events not stored there. The
/// `address` column the join also returns is unused (the `Event` object is
/// per-event, not per-participant), so it is simply not a field here.
#[derive(FromQueryResult)]
pub(super) struct EventRow {
    pub block_height: i64,
    pub category: i16,
    pub category_index: i32,
    pub event_index: i32,
    pub event_type: i16,
    pub contract: Option<Vec<u8>>,
    pub contract_event_name: Option<String>,
    pub data: Option<Vec<u8>>,
}

/// An indexed event, identified by its position. The participant set is **not**
/// a field — an event may have several participants; the address feeds (queries
/// 5–8) return one row per (event × address) and the attribute feeds (queries
/// 2–4) collapse to the event. `contract` / `name` are present only for
/// contract events; the decoded `data` payload is hydrated on demand.
#[derive(SimpleObject, Clone, Debug)]
#[graphql(complex)]
pub struct Event {
    /// Height of the block the event was emitted in.
    pub block_height: u64,
    /// Kind of the enclosing unit (a transaction or a cronjob).
    pub category: UnitKind,
    /// Index of the enclosing unit within the block.
    pub category_index: u32,
    /// The event's 0-based position within its unit.
    pub event_index: u32,
    /// The event's type.
    #[graphql(name = "type")]
    pub ty: EventType,
    /// Emitting contract — present only for contract events.
    pub contract: Option<Address>,
    /// The contract-event name (`order_filled`, …) — present only for contract
    /// events.
    pub name: Option<String>,
    /// Compressed payload carried by the feed's `event_data` join (priority
    /// types); `None` for non-priority events, hydrated from the block instead.
    /// Not a GraphQL field — surfaced decoded through [`data`](Self::data).
    #[graphql(skip)]
    raw_data: Option<Vec<u8>>,
}

impl TryFrom<EventRow> for Event {
    type Error = Error;

    fn try_from(row: EventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            block_height: row.block_height as u64,
            category: UnitKind::from_code(row.category)?,
            category_index: row.category_index as u32,
            event_index: row.event_index as u32,
            ty: EventType::from_code(row.event_type)
                .ok_or_else(|| Error::new(format!("unknown event_type: {}", row.event_type)))?,
            contract: row.contract.map(address_from_bytes).transpose()?,
            name: row.contract_event_name,
            raw_data: row.data,
        })
    }
}

#[ComplexObject]
impl Event {
    /// The event's decoded payload as JSON — a `FlatEvent`, e.g. `{"transfer":
    /// …}` / `{"execute": …}`. Resolved only when selected: for priority types
    /// the payload rode along with the feed query (the `event_data` join), so it
    /// is just decompressed here; for non-priority types it is hydrated from the
    /// unit's block (shared `BlockLoader`) and flattened. `null` only if the
    /// block is no longer available from the source.
    ///
    /// Cost note: selecting `data` on a feed of a **non-priority** type (kept by
    /// `event_type_filter` but absent from `event_data_filter`, e.g. `execute`)
    /// pays one block load per *distinct* block in the page. The `BlockLoader`
    /// dedups by height, so a type with many events per block costs only a few
    /// reads, while a sparse type over cold history approaches one read per row
    /// (bounded by `MAX_LIMIT`). If a deployment queries such payloads often, add
    /// the type to `event_data_filter` (and re-backfill) so they are stored
    /// inline and the feed never hydrates.
    async fn data(&self, ctx: &Context<'_>) -> Result<Option<Json<FlatEvent>>> {
        // Priority types: the payload came with the feed row — just decode it.
        if let Some(blob) = &self.raw_data {
            let event = decompress_event(blob)
                .map_err(|err| Error::new(format!("failed to decode event payload: {err}")))?;
            return Ok(Some(Json(event)));
        }
        // Non-priority types: absent from `event_data`; flatten from the block.
        let loader = ctx.data_unchecked::<DataLoader<BlockLoader>>();
        let Some(block) = loader.load_one(self.block_height).await? else {
            return Ok(None);
        };
        let flat = flatten_unit(&block, self.category.code(), self.category_index as usize);
        // Match on the stored `event_index` *value*, not the Vec position. The
        // two coincide today (the flatten numbers a unit's events `0..n`
        // contiguously — the same value the write path stored), but looking it
        // up by id keeps this correct even if that numbering ever changes. `n`
        // is the unit's event count, so the scan is tiny.
        let info = flat
            .iter()
            .find(|info| info.id.event_index == self.event_index)
            .ok_or_else(|| {
                Error::new(format!(
                    "event {} missing from unit ({}, {}) of block {}",
                    self.event_index,
                    self.category.code(),
                    self.category_index,
                    self.block_height
                ))
            })?;
        Ok(Some(Json(info.event.clone())))
    }
}

/// An executed unit — a transaction (`kind = TRANSACTION`) or a cronjob
/// (`kind = CRON`). `hash` and `sender` are absent for cron units. The indexed
/// columns are cheap scalar fields; the full submitted payload and the
/// execution outcome (gas, result, events) are the on-demand `tx` / `outcome`
/// fields, hydrated from the unit's block only when selected.
#[derive(SimpleObject, Clone, Debug)]
#[graphql(complex)]
pub struct Transaction {
    /// Height of the block the unit executed in.
    pub block_height: u64,
    /// Index of the unit within the block (per kind).
    pub idx: u32,
    /// Whether this is a transaction or a cronjob.
    pub kind: UnitKind,
    /// Content hash — absent for cron units.
    pub hash: Option<Hash>,
    /// Account or contract that sent the unit — absent for cron units.
    pub sender: Option<Address>,
    /// Whether the unit's outcome was `Ok`.
    pub success: bool,
    /// Block time, RFC 3339 / ISO 8601 (UTC). A string, not an integer, so the
    /// nanosecond value never loses precision in 64-bit-lossy clients.
    pub timestamp: String,
}

impl TryFrom<transactions::Model> for Transaction {
    type Error = Error;

    fn try_from(model: transactions::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            block_height: model.block_height as u64,
            idx: model.idx as u32,
            kind: UnitKind::from_code(model.kind)?,
            hash: model.hash.map(hash_from_bytes).transpose()?,
            sender: model.sender.map(address_from_bytes).transpose()?,
            success: model.success,
            timestamp: Timestamp::from_nanos(model.timestamp as u128).to_rfc3339_string(),
        })
    }
}

/// A unit's execution outcome, as carried in its block: a [`TxOutcome`] for a
/// transaction, a [`CronOutcome`] for a cronjob. Serialized to JSON for the
/// `outcome` field, externally tagged — `{"transaction": …}` / `{"cron": …}` —
/// so the payload says which it is, alongside the unit's `kind`.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum UnitOutcome {
    // Boxed: a `TxOutcome` (with its full event tree) dwarfs a `CronOutcome`, so
    // an unboxed enum would carry the larger everywhere. Serialization is
    // transparent through the `Box`.
    Transaction(Box<TxOutcome>),
    Cron(Box<CronOutcome>),
}

#[ComplexObject]
impl Transaction {
    /// The full transaction as submitted on-chain — sender, messages,
    /// credential, data — hydrated on demand from the unit's block. `null` for
    /// cron units, which have no transaction. Returned as the canonical `Tx`
    /// JSON (the same shape the node serializes).
    async fn tx(&self, ctx: &Context<'_>) -> Result<Option<Tx>> {
        // A cronjob has no transaction; skip the block load entirely.
        if self.kind != UnitKind::Transaction {
            return Ok(None);
        }
        let block = self.block(ctx).await?;
        let (tx, _hash) = block
            .block
            .txs
            .get(self.idx as usize)
            .ok_or_else(|| self.missing("transaction"))?;
        Ok(Some(tx.clone()))
    }

    /// The unit's execution outcome — gas, success / error, the event tree —
    /// recorded in its block, as JSON: a `TxOutcome` for a transaction, a
    /// `CronOutcome` for a cronjob (see [`UnitOutcome`]). Hydrated like
    /// [`tx`](Self::tx), sharing its one block read. Wrapped in `Json` because
    /// the outcome types render as JSON (and `TxOutcome`'s own GraphQL output
    /// type would otherwise leave a dangling SDL reference).
    async fn outcome(&self, ctx: &Context<'_>) -> Result<Option<Json<UnitOutcome>>> {
        let block = self.block(ctx).await?;
        let idx = self.idx as usize;
        let outcome = match self.kind {
            UnitKind::Transaction => UnitOutcome::Transaction(Box::new(
                block
                    .outcome
                    .tx_outcomes
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| self.missing("transaction outcome"))?,
            )),
            UnitKind::Cron => UnitOutcome::Cron(Box::new(
                block
                    .outcome
                    .cron_outcomes
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| self.missing("cron outcome"))?,
            )),
        };
        Ok(Some(Json(outcome)))
    }
}

impl Transaction {
    /// Load this unit's block through the shared [`BlockLoader`]. The loader
    /// batches and dedups by height, so `tx` and `outcome` on the same unit (and
    /// every row sharing a block in the page) cost one block read between them.
    /// Errors if the source no longer has the block.
    async fn block(&self, ctx: &Context<'_>) -> Result<Arc<BlockData>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockLoader>>();
        loader.load_one(self.block_height).await?.ok_or_else(|| {
            Error::new(format!(
                "block {} not available from source",
                self.block_height
            ))
        })
    }

    /// A "`<what>` N missing from block H" error for a payload the block should
    /// have held at this unit's index — a data-integrity failure, not a routine
    /// absence.
    fn missing(&self, what: &str) -> Error {
        Error::new(format!(
            "{what} {} missing from block {}",
            self.idx, self.block_height
        ))
    }
}
