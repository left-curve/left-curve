//! The activity projection's GraphQL **type surface**: the blockchain-primitive
//! scalars (`Address`, `Hash`) and the output objects (`Transaction`, `Event`)
//! the read resolvers return. The resolvers themselves live in [`super::query`];
//! the keyset machinery in [`super::pagination`].
//!
//! Addresses and hashes are stored as raw bytes (`BYTEA`) but exposed as their
//! canonical text form via thin newtypes over grug's `Addr` / `Hash256` — so
//! the API speaks the same hex dialect as the rest of dango: an `Address` is
//! lowercase `0x`-prefixed, a `Hash` is bare uppercase. The newtypes exist
//! because the scalar trait and those primitives are both foreign here.

use {
    crate::activity::{
        entity::{events, transactions},
        event_type::EventType,
    },
    async_graphql::{
        Enum, Error, InputValueError, InputValueResult, Scalar, ScalarType, SimpleObject, Value,
    },
    dango_primitives::{Addr, FlatCategory, Hash256, Timestamp},
    std::str::FromStr,
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

/// An indexed event, identified by its position. The participant set is **not**
/// a field — an event may have several participants; the address feeds (queries
/// 5–8) return one row per (event × address) and the attribute feeds (queries
/// 2–4) collapse to the event. `contract` / `name` are present only for
/// contract events.
#[derive(SimpleObject, Clone, Debug)]
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
}

impl TryFrom<events::Model> for Event {
    type Error = Error;

    fn try_from(model: events::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            block_height: model.block_height as u64,
            category: UnitKind::from_code(model.category)?,
            category_index: model.category_index as u32,
            event_index: model.event_index as u32,
            ty: EventType::from_code(model.event_type)
                .ok_or_else(|| Error::new(format!("unknown event_type: {}", model.event_type)))?,
            contract: model.contract.map(address_from_bytes).transpose()?,
            name: model.contract_event_name,
        })
    }
}

/// An executed unit — a transaction (`kind = TRANSACTION`) or a cronjob
/// (`kind = CRON`). `hash`, `sender`, and `gasLimit` are absent for cron units.
#[derive(SimpleObject, Clone, Debug)]
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
    /// Gas limit — absent for cron units (unlimited).
    pub gas_limit: Option<u64>,
    /// Gas actually consumed.
    pub gas_used: u64,
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
            gas_limit: model.gas_limit.map(|gas| gas as u64),
            gas_used: model.gas_used as u64,
            timestamp: Timestamp::from_nanos(model.timestamp as u128).to_rfc3339_string(),
        })
    }
}
