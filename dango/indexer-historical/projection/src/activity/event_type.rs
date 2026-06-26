use {
    async_graphql::Enum,
    dango_primitives::{Addr, FlatEvent},
    serde::{Deserialize, Serialize},
};

/// Compact discriminant for a [`FlatEvent`], stored in `activity_events.event_type`
/// and used to key the projection's priority / involvement configuration.
///
/// Mirrors `FlatEvent`'s variants one-to-one. The `#[repr(i16)]` values are the
/// **stored codes**: they are part of the on-disk schema, so existing codes must
/// never be renumbered (append-only). [`From<&FlatEvent>`] is exhaustive, so a
/// new upstream variant is a compile error here until it is given a code.
///
/// Also the GraphQL enum used to filter the activity feeds by type (queries 2 /
/// 6) and to surface an event's type in the read API.
#[derive(Enum, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
pub enum EventType {
    Configure = 0,
    Upgrade = 1,
    Transfer = 2,
    Upload = 3,
    Instantiate = 4,
    Execute = 5,
    Migrate = 6,
    Reply = 7,
    Authenticate = 8,
    Backrun = 9,
    Withhold = 10,
    Finalize = 11,
    Cron = 12,
    Guest = 13,
    ContractEvent = 14,
}

impl EventType {
    /// The stored discriminant (the value written to `event_type`).
    #[must_use]
    pub fn code(self) -> i16 {
        self as i16
    }

    /// The variant for a stored discriminant, or `None` if `code` is unknown —
    /// the inverse of [`code`](Self::code), used to surface `event_type` in the
    /// read API. Spelled out (not `transmute`) so an out-of-range value read
    /// from the database is a clean `None`, never undefined behaviour.
    #[must_use]
    pub fn from_code(code: i16) -> Option<Self> {
        Some(match code {
            0 => Self::Configure,
            1 => Self::Upgrade,
            2 => Self::Transfer,
            3 => Self::Upload,
            4 => Self::Instantiate,
            5 => Self::Execute,
            6 => Self::Migrate,
            7 => Self::Reply,
            8 => Self::Authenticate,
            9 => Self::Backrun,
            10 => Self::Withhold,
            11 => Self::Finalize,
            12 => Self::Cron,
            13 => Self::Guest,
            14 => Self::ContractEvent,
            _ => return None,
        })
    }
}

impl From<&FlatEvent> for EventType {
    fn from(event: &FlatEvent) -> Self {
        match event {
            FlatEvent::Configure(_) => Self::Configure,
            FlatEvent::Upgrade(_) => Self::Upgrade,
            FlatEvent::Transfer(_) => Self::Transfer,
            FlatEvent::Upload(_) => Self::Upload,
            FlatEvent::Instantiate(_) => Self::Instantiate,
            FlatEvent::Execute(_) => Self::Execute,
            FlatEvent::Migrate(_) => Self::Migrate,
            FlatEvent::Reply(_) => Self::Reply,
            FlatEvent::Authenticate(_) => Self::Authenticate,
            FlatEvent::Backrun(_) => Self::Backrun,
            FlatEvent::Withhold(_) => Self::Withhold,
            FlatEvent::Finalize(_) => Self::Finalize,
            FlatEvent::Cron(_) => Self::Cron,
            FlatEvent::Guest(_) => Self::Guest,
            FlatEvent::ContractEvent(_) => Self::ContractEvent,
        }
    }
}

// ---- per-event taxonomy used by the `events` table columns ----

/// The contract that emitted a contract event — the value of
/// `activity_events.contract`. Populated **only for `ContractEvent`**: the
/// contract feeds (queries 3/4/7/8) always filter `contract` together with
/// `type = ContractEvent`, so `contract IS NOT NULL` is made to mean exactly
/// "a contract event", and the other event kinds that happen to carry a
/// contract are deliberately left out (reversible — re-backfill if an "Execute
/// / Migrate by contract" axis is ever wanted). `None` for every other event.
pub(crate) fn event_contract(event: &FlatEvent) -> Option<Addr> {
    match event {
        FlatEvent::ContractEvent(e) => Some(e.contract),
        _ => None,
    }
}

/// The contract-event type string (`CheckedContractEvent.ty`) for the
/// `contract_event_name` column and query 4. `None` for every
/// non-contract-event flat event.
pub(crate) fn contract_event_name(event: &FlatEvent) -> Option<String> {
    match event {
        FlatEvent::ContractEvent(e) => Some(e.ty.clone()),
        _ => None,
    }
}
