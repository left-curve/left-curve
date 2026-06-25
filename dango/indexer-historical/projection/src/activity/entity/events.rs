use sea_orm::entity::prelude::*;

/// The merged event log + participation index — one row per **(event ×
/// participant address)**. This single table replaces the former separate
/// `events` and `involvement` tables: their columns were near-identical, and
/// (after the event-type blacklist drops the address-less noise) most surviving
/// events carry an address, so storing each event once *with* its participant
/// is leaner than storing it in two tables.
///
/// Row kinds, all keyed by `(address, block_height, category, category_index,
/// event_index)`:
///
/// - **participation** — `address` = a 20-byte participant, `event_index >= 0`.
///   An event with K participants has K such rows (K is usually 1).
/// - **address-less event** — `address` = the **empty byte string** (a real
///   address is always 20 bytes, so it can never collide), `event_index >= 0`.
///   One per kept event that has no (non-blacklisted) participant, so the
///   attribute feeds (by type / contract / name) never lose an event.
/// - **sentinel** — `address` = the tx sender, `event_index = -1`,
///   `event_type` / `contract` / `contract_event_name` all NULL. Lets "txs
///   involving X" surface a unit where X is only the sender.
///
/// The canonical *event* is the position `(block_height, category,
/// category_index, event_index)`; recover distinct events with `DISTINCT ON`
/// (a no-op at K = 1, correct for K > 1). The payload lives in
/// [`super::event_data`] for priority types only.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "activity_events")]
pub struct Model {
    /// Participant (20 bytes); empty = address-less event; sender on the
    /// sentinel.
    #[sea_orm(primary_key, auto_increment = false)]
    pub address: Vec<u8>,
    #[sea_orm(primary_key, auto_increment = false)]
    pub block_height: i64,
    /// The unit's kind ([`dango_primitives::FlatCategory`]): 0 = cron, 1 = tx.
    #[sea_orm(primary_key, auto_increment = false)]
    pub category: i16,
    /// The tx / cron index within the block.
    #[sea_orm(primary_key, auto_increment = false)]
    pub category_index: i32,
    /// The event's position within the unit (0-based); `-1` on the sentinel.
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_index: i32,
    /// [`crate::EventType`] discriminant; NULL only on the sentinel.
    pub event_type: Option<i16>,
    /// Emitting / subject contract (20 bytes); NULL where not applicable.
    pub contract: Option<Vec<u8>>,
    /// The contract-event `ty` (e.g. `order_filled`); NULL otherwise.
    pub contract_event_name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
