//! The activity read API's **type surface**: the `UnitKind` / `AddressRole`
//! enums used as arguments, and the `Transaction` / `Event` JSON objects the
//! handlers return.
//!
//! Addresses and hashes are grug's own `Addr` / `Hash256`: their serde already
//! speaks the canonical text dialect — an address is lowercase `0x`-prefixed, a
//! hash bare uppercase — both on input (`web::Path<Addr>` parses the hex, like
//! `web::Path<Uuid>`) and output (serialized as that same string), so the read
//! API needs no wrapper newtype. Stored as raw bytes (`BYTEA`), they are decoded
//! back with `Addr::try_from` / `Hash256::try_from`.
//!
//! The heavy detail — a unit's full `tx` / `outcome`, an event's decoded `data`
//! — is **always** hydrated eagerly from the unit's block (see [`super::hydrate`])
//! and lands in the `tx` / `outcome` / `data` fields; the feeds in
//! [`super::feeds`] leave them `None` and the handler fills them before
//! responding.

use {
    crate::activity::{decompress_event, entity::transactions, event_type::EventType},
    dango_indexer_historical_httpd::ApiError,
    dango_primitives::{Addr, Hash256, Timestamp},
    sea_orm::FromQueryResult,
    serde::{Deserialize, Serialize},
};

// ---- stored-bytes decoders ----

/// `Addr` from stored bytes — a stored address is always 20 bytes, so a
/// wrong-length value is a data-integrity error surfaced as a 500.
fn addr_from_bytes(bytes: Vec<u8>) -> Result<Addr, ApiError> {
    Addr::try_from(bytes)
        .map_err(|err| ApiError::Internal(format!("invalid stored address: {err}")))
}

/// `Hash256` from stored bytes — same contract as [`addr_from_bytes`].
fn hash_from_bytes(bytes: Vec<u8>) -> Result<Hash256, ApiError> {
    Hash256::try_from(bytes)
        .map_err(|err| ApiError::Internal(format!("invalid stored hash: {err}")))
}

// ---- enums ----

/// How an `address` argument must relate to a unit in the
/// `transactionsInvolving` feed. Omitted ⇒ **either** — the address sent the
/// unit *or* was a party to one of its events (their union).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AddressRole {
    /// The address sent the unit (`transactions.sender`). Cron units have no
    /// sender, so this never matches a cronjob.
    Sender,
    /// The address is a party to one of the unit's events (the participation
    /// rows in `events`).
    Participant,
}

/// The kind of an executed unit — mirrors [`dango_primitives::FlatCategory`].
/// Serialized / parsed as `"cron"` / `"transaction"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnitKind {
    Cron,
    Transaction,
}

impl UnitKind {
    /// The stored discriminant for this kind (the value in `transactions.kind`
    /// / `events.category`).
    pub(crate) fn code(self) -> i16 {
        match self {
            Self::Cron => dango_primitives::FlatCategory::Cron as i16,
            Self::Transaction => dango_primitives::FlatCategory::Tx as i16,
        }
    }

    fn from_code(code: i16) -> Result<Self, ApiError> {
        match code {
            c if c == dango_primitives::FlatCategory::Cron as i16 => Ok(Self::Cron),
            c if c == dango_primitives::FlatCategory::Tx as i16 => Ok(Self::Transaction),
            other => Err(ApiError::Internal(format!("unknown unit kind: {other}"))),
        }
    }
}

// ---- event rows + output ----

/// One event feed row: the event's stored columns plus its payload blob from
/// the `event_data` join — `None` for non-priority events not stored there. The
/// `address` column the join also returns is unused (the `Event` object is
/// per-event, not per-participant), so it is simply not a field here.
#[derive(FromQueryResult)]
pub(crate) struct EventRow {
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
/// a field — the address feeds return one row per (event × address) and the
/// attribute feeds collapse to the event. `contract` / `name` are present only
/// for contract events; `data` is the decoded payload, hydrated eagerly (from
/// the `event_data` blob for priority types, otherwise from the unit's block),
/// `null` only if the source no longer holds the block.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Event {
    /// Height of the block the event was emitted in.
    pub block_height: u64,
    /// Kind of the enclosing unit (a transaction or a cronjob).
    pub category: UnitKind,
    /// Index of the enclosing unit within the block.
    pub category_index: u32,
    /// The event's 0-based position within its unit.
    pub event_index: u32,
    /// The event's type.
    #[serde(rename = "type")]
    pub ty: EventType,
    /// Emitting contract — present only for contract events.
    pub contract: Option<Addr>,
    /// The contract-event name (`order_filled`, …) — present only for contract
    /// events.
    pub name: Option<String>,
    /// The decoded payload as JSON — a `FlatEvent`. Set by [`event_from_row`]
    /// for priority types (decompressed from the joined blob) and by
    /// [`super::hydrate`] from the block for the rest; `null` when the block is
    /// unavailable.
    pub data: Option<serde_json::Value>,
}

/// Build an [`Event`] from a feed row, decoding the priority payload that rode
/// along in the `event_data` join. Non-priority rows (no joined blob) leave
/// `data` `None` for [`super::hydrate`] to fill from the block.
pub(crate) fn event_from_row(row: EventRow) -> Result<Event, ApiError> {
    let data = match &row.data {
        Some(blob) => Some(serde_json::to_value(decompress_event(blob)?)?),
        None => None,
    };
    Ok(Event {
        block_height: row.block_height as u64,
        category: UnitKind::from_code(row.category)?,
        category_index: row.category_index as u32,
        event_index: row.event_index as u32,
        ty: EventType::from_code(row.event_type)
            .ok_or_else(|| ApiError::Internal(format!("unknown event_type: {}", row.event_type)))?,
        contract: row.contract.map(addr_from_bytes).transpose()?,
        name: row.contract_event_name,
        data,
    })
}

/// An executed unit — a transaction (`kind = "transaction"`) or a cronjob
/// (`kind = "cron"`). `hash` and `sender` are absent for cron units. The indexed
/// columns are cheap; the full submitted `tx` and the execution `outcome` are
/// hydrated eagerly from the unit's block (see [`super::hydrate`]) — `tx` is
/// `null` for cron units, both are `null` if the block is unavailable.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Transaction {
    /// Height of the block the unit executed in.
    pub block_height: u64,
    /// Index of the unit within the block (per kind).
    pub idx: u32,
    /// Whether this is a transaction or a cronjob.
    pub kind: UnitKind,
    /// Content hash — absent for cron units.
    pub hash: Option<Hash256>,
    /// Account or contract that sent the unit — absent for cron units.
    pub sender: Option<Addr>,
    /// Whether the unit's outcome was `Ok`.
    pub success: bool,
    /// Block time, RFC 3339 / ISO 8601 (UTC). A string, not an integer, so the
    /// nanosecond value never loses precision in 64-bit-lossy clients.
    pub timestamp: String,
    /// The full transaction as submitted on-chain (the `Tx` JSON the node
    /// serializes) — `null` for cron units. Hydrated from the block.
    pub tx: Option<serde_json::Value>,
    /// The unit's execution outcome as JSON — externally tagged
    /// `{"transaction": …}` / `{"cron": …}` (see [`UnitOutcome`]). Hydrated from
    /// the block.
    pub outcome: Option<serde_json::Value>,
}

impl TryFrom<transactions::Model> for Transaction {
    type Error = ApiError;

    fn try_from(model: transactions::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            block_height: model.block_height as u64,
            idx: model.idx as u32,
            kind: UnitKind::from_code(model.kind)?,
            hash: model.hash.map(hash_from_bytes).transpose()?,
            sender: model.sender.map(addr_from_bytes).transpose()?,
            success: model.success,
            timestamp: Timestamp::from_nanos(model.timestamp as u128).to_rfc3339_string(),
            tx: None,
            outcome: None,
        })
    }
}

/// A unit's execution outcome, as carried in its block: a [`TxOutcome`] for a
/// transaction, a [`CronOutcome`] for a cronjob. Serialized externally tagged —
/// `{"transaction": …}` / `{"cron": …}` — so the payload says which it is,
/// alongside the unit's `kind`. Built by [`super::hydrate`].
///
/// [`TxOutcome`]: dango_primitives::TxOutcome
/// [`CronOutcome`]: dango_primitives::CronOutcome
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnitOutcome {
    // Boxed: a `TxOutcome` (with its full event tree) dwarfs a `CronOutcome`, so
    // an unboxed enum would carry the larger everywhere. Serialization is
    // transparent through the `Box`.
    Transaction(Box<dango_primitives::TxOutcome>),
    Cron(Box<dango_primitives::CronOutcome>),
}
