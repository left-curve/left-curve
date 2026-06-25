use {
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
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// The contract a flat event is "about" — its subject / emitter — for the
/// `activity_events.contract` column and query 6 ("events emitted by contract
/// C"). `None` for events with no associated contract (transfers, the
/// authenticate / withhold wrappers, configure / upgrade / upload).
pub(crate) fn event_contract(event: &FlatEvent) -> Option<Addr> {
    match event {
        FlatEvent::ContractEvent(e) => Some(e.contract),
        FlatEvent::Execute(e) => Some(e.contract),
        FlatEvent::Instantiate(e) => Some(e.contract),
        FlatEvent::Migrate(e) => Some(e.contract),
        FlatEvent::Reply(e) => Some(e.contract),
        FlatEvent::Cron(e) => Some(e.contract),
        FlatEvent::Guest(e) => Some(e.contract),
        FlatEvent::Configure(_)
        | FlatEvent::Upgrade(_)
        | FlatEvent::Transfer(_)
        | FlatEvent::Upload(_)
        | FlatEvent::Authenticate(_)
        | FlatEvent::Backrun(_)
        | FlatEvent::Withhold(_)
        | FlatEvent::Finalize(_) => None,
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
