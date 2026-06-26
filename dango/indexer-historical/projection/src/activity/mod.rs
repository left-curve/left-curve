//! The activity projection — the foundational projection of the historical
//! indexer. For every block it writes three Postgres tables that together
//! answer, for any account address, *"what did it do and what touched it"*:
//!
//! - [`entity::transactions`] — one row per executed unit (a tx or a cronjob);
//! - [`entity::events`] — the merged event log + participation index: one row
//!   per (event × participant address), with an empty-address row for kept
//!   events that have no participant;
//! - [`entity::event_data`] — the event payload, split out and kept only for
//!   priority types.
//!
//! A single [`process`](ActivityProjection::process) flattens the block's
//! events once and stages all three tables into one [`Ctx`], so they commit
//! atomically with the projection's cursor. Every key is the deterministic
//! position (+ address) and every insert is `ON CONFLICT DO NOTHING`, so a
//! post-crash replay of a block is a no-op. See `DESIGN.md` in this folder.

mod entity;
mod event_type;
mod graphql;
mod idens;
mod migrations;

#[cfg(feature = "tracing")]
use tracing::instrument;
use {
    crate::{Ctx, Projection},
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    dango_primitives::{
        Addr, EventId, Extractable, FlatCategory, FlatEvent, FlatEventInfo,
        flatten_commitment_status, flatten_tx_events,
    },
    entity::{event_data, events, transactions},
    event_type::{contract_event_name, event_contract},
    sea_orm::{ActiveValue::Set, EntityTrait, sea_query::OnConflict},
    sea_orm_migration::MigrationTrait,
    std::collections::HashSet,
};
pub use {event_type::EventType, graphql::ActivityQuery};

/// Stable projection id keying the cursor row. Bumping it forces a full
/// re-backfill.
const PROJECTION_ID: &str = "activity";

/// zstd level for the `event_data.data` column. 3 is zstd's own default — a
/// good ratio/speed trade-off for the borsh-encoded event payloads.
const ZSTD_LEVEL: i32 = 3;

// ---- configuration ----

/// Write-time knobs for the activity projection. Applied as each block is
/// processed, so changing them is **not** retroactive (see `DESIGN.md` §
/// Configuration): re-populating already-written rows needs a re-backfill.
#[derive(Clone, Debug)]
pub struct ActivityConfig {
    /// Event types dropped entirely — no `events` row, no payload. The
    /// address-less system noise (guest/withhold/authenticate/finalize/…) that
    /// dominates the raw stream and is not user-facing activity.
    pub event_type_blacklist: HashSet<EventType>,
    /// Event types whose payload (`zstd(borsh(event))`) is stored in
    /// `event_data`; others are hydrated from the raw block on demand.
    pub event_data_types: HashSet<EventType>,
    /// Event types whose participants are extracted into the `events` rows
    /// (one row per participant). Other kept types get a single empty-address
    /// row.
    pub involvement_types: HashSet<EventType>,
    /// Addresses excluded from participation at write time — the deployment's
    /// system contracts. Deployment-specific, so empty by default and
    /// populated from config once the CLI is wired.
    pub involvement_blacklist: HashSet<Addr>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        let priority = HashSet::from([EventType::Transfer, EventType::ContractEvent]);
        Self {
            // The pure plumbing: emitted on every call/tx, never carries a
            // meaningful participant. Measured ~81% of the raw mainnet stream.
            event_type_blacklist: HashSet::from([
                EventType::Guest,
                EventType::Withhold,
                EventType::Authenticate,
                EventType::Finalize,
                EventType::Backrun,
                EventType::Reply,
            ]),
            event_data_types: priority.clone(),
            involvement_types: priority,
            involvement_blacklist: HashSet::new(),
        }
    }
}

// ---- projection ----

/// See the [module docs](self).
#[derive(Clone, Debug, Default)]
pub struct ActivityProjection {
    config: ActivityConfig,
}

impl ActivityProjection {
    #[must_use]
    pub fn new(config: ActivityConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Projection for ActivityProjection {
    fn id(&self) -> &'static str {
        PROJECTION_ID
    }

    fn min_height(&self) -> u64 {
        0
    }

    fn migrations(&self) -> Vec<Box<dyn MigrationTrait>> {
        migrations::migrations()
    }

    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(height = block.block.info.height)))]
    async fn process(&self, ctx: &mut Ctx, block: &BlockData) -> AnyResult<()> {
        let block_height = block.block.info.height;
        let height = block_height as i64;
        let timestamp = block.block.info.timestamp.into_nanos() as i64;

        let mut rows = Rows::default();

        // Each unit is flattened on its own `EventId`, so `event_index` is
        // 0-based within the unit; `(block_height, category, category_index,
        // event_index)` is the canonical event position, unique because
        // `category_index` separates the units.

        // ---- cron units ----
        for (cron_idx, cron_outcome) in block.outcome.cron_outcomes.iter().enumerate() {
            rows.transactions.push(transactions::ActiveModel {
                block_height: Set(height),
                idx: Set(cron_idx as i32),
                kind: Set(FlatCategory::Cron as i16),
                hash: Set(None),
                sender: Set(None),
                success: Set(cron_outcome.cron_event.maybe_error().is_none()),
                gas_limit: Set(cron_outcome.gas_limit.map(|g| g as i64)),
                gas_used: Set(cron_outcome.gas_used as i64),
                timestamp: Set(timestamp),
            });

            // A cronjob is a single commitment group, flattened on a fresh
            // per-unit id. Cron has no sender, so no `transactions.sender` row
            // to feed query 1's sender side — only its participants.
            let mut event_id = EventId::new(block_height, FlatCategory::Cron, cron_idx as u32, 0);
            let flat = flatten_commitment_status(&mut event_id, cron_outcome.cron_event.clone());
            self.push_events(&mut rows, &flat, height)?;
        }

        // ---- tx units ----
        for (tx_idx, ((tx, tx_hash), tx_outcome)) in block
            .block
            .txs
            .iter()
            .zip(block.outcome.tx_outcomes.iter())
            .enumerate()
        {
            rows.transactions.push(transactions::ActiveModel {
                block_height: Set(height),
                idx: Set(tx_idx as i32),
                kind: Set(FlatCategory::Tx as i16),
                hash: Set(Some(tx_hash.as_ref().to_vec())),
                sender: Set(Some(tx.sender.as_ref().to_vec())),
                success: Set(tx_outcome.result.is_ok()),
                gas_limit: Set(Some(tx.gas_limit as i64)),
                gas_used: Set(tx_outcome.gas_used as i64),
                timestamp: Set(timestamp),
            });

            // `flatten_tx_events` flattens the four commitment groups (withhold,
            // authenticate, msgs + backrun, finalize) on a fresh per-tx id.
            let flat = flatten_tx_events(tx_outcome.events.clone(), block_height, tx_idx as u32);
            self.push_events(&mut rows, &flat, height)?;
        }

        #[cfg(feature = "metrics")]
        {
            metrics::counter!("indexer_historical_activity_transactions_total")
                .increment(rows.transactions.len() as u64);
            metrics::counter!("indexer_historical_activity_events_total")
                .increment(rows.events.len() as u64);
        }

        rows.write(ctx).await
    }
}

impl ActivityProjection {
    /// Stage every flattened event of one unit.
    fn push_events(&self, rows: &mut Rows, flat: &[FlatEventInfo], height: i64) -> AnyResult<()> {
        for info in flat {
            self.push_event(rows, info, height)?;
        }

        Ok(())
    }

    /// Stage one flattened event into the merged `events` table: one row per
    /// participant (or a single empty-address row if it has none), plus the
    /// payload into `event_data` for priority types. Blacklisted types are
    /// dropped wholesale.
    fn push_event(&self, rows: &mut Rows, info: &FlatEventInfo, height: i64) -> AnyResult<()> {
        let event = &info.event;
        let event_type = EventType::from(event);

        if self.config.event_type_blacklist.contains(&event_type) {
            return Ok(());
        }

        let category = info.id.category as i16;
        let category_index = info.id.category_index as i32;
        let event_index = info.id.event_index as i32;

        let contract = event_contract(event).map(|a| a.as_ref().to_vec());
        let contract_event_name = contract_event_name(event);

        // Payload into the side-table, only for priority types.
        if self.config.event_data_types.contains(&event_type) {
            rows.event_data.push(event_data::ActiveModel {
                block_height: Set(height),
                category: Set(category),
                category_index: Set(category_index),
                event_index: Set(event_index),
                data: Set(compress_event(event)?),
            });
        }

        // Participants for the configured involvement types, minus the
        // blacklist. An event with no participant still gets one row, keyed by
        // the empty address, so the attribute feeds never lose it.
        let mut addresses: Vec<Vec<u8>> = if self.config.involvement_types.contains(&event_type) {
            let mut set = HashSet::new();
            event.extract_addresses(&mut set);
            set.into_iter()
                .filter(|a| !self.config.involvement_blacklist.contains(a))
                .map(|a| a.as_ref().to_vec())
                .collect()
        } else {
            Vec::new()
        };
        if addresses.is_empty() {
            addresses.push(Vec::new()); // the empty-address marker
        }

        for address in addresses {
            rows.events.push(events::ActiveModel {
                address: Set(address),
                block_height: Set(height),
                category: Set(category),
                category_index: Set(category_index),
                event_index: Set(event_index),
                event_type: Set(event_type.code()),
                contract: Set(contract.clone()),
                contract_event_name: Set(contract_event_name.clone()),
            });
        }

        Ok(())
    }
}

// ---- staged writes ----

/// One block's staged rows, flushed together in [`Rows::write`].
#[derive(Default)]
struct Rows {
    transactions: Vec<transactions::ActiveModel>,
    events: Vec<events::ActiveModel>,
    event_data: Vec<event_data::ActiveModel>,
}

impl Rows {
    async fn write(self, ctx: &mut Ctx) -> AnyResult<()> {
        let txn = ctx.pg();

        if !self.transactions.is_empty() {
            transactions::Entity::insert_many(self.transactions)
                .on_conflict(
                    OnConflict::columns([
                        transactions::Column::BlockHeight,
                        transactions::Column::Idx,
                        transactions::Column::Kind,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec_without_returning(txn)
                .await?;
        }

        if !self.events.is_empty() {
            events::Entity::insert_many(self.events)
                .on_conflict(
                    OnConflict::columns([
                        events::Column::Address,
                        events::Column::BlockHeight,
                        events::Column::Category,
                        events::Column::CategoryIndex,
                        events::Column::EventIndex,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec_without_returning(txn)
                .await?;
        }

        if !self.event_data.is_empty() {
            event_data::Entity::insert_many(self.event_data)
                .on_conflict(
                    OnConflict::columns([
                        event_data::Column::BlockHeight,
                        event_data::Column::Category,
                        event_data::Column::CategoryIndex,
                        event_data::Column::EventIndex,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec_without_returning(txn)
                .await?;
        }

        Ok(())
    }
}

/// `zstd(borsh(event))` — the stored form of a priority event's payload.
fn compress_event(event: &FlatEvent) -> AnyResult<Vec<u8>> {
    let borshed = borsh::to_vec(event)?;
    let compressed = zstd::encode_all(borshed.as_slice(), ZSTD_LEVEL)?;
    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use {super::*, dango_primitives::FlatEvtBackrun};

    #[test]
    fn event_type_codes_are_stable() {
        // Codes are part of the on-disk schema — pin them so a reorder can't
        // silently renumber stored rows.
        assert_eq!(EventType::Configure.code(), 0);
        assert_eq!(EventType::Transfer.code(), 2);
        assert_eq!(EventType::Execute.code(), 5);
        assert_eq!(EventType::ContractEvent.code(), 14);

        // `from_code` is the exact inverse used to surface `event_type` in the
        // read API, and rejects out-of-range codes.
        for code in 0..=14i16 {
            assert_eq!(EventType::from_code(code).map(EventType::code), Some(code));
        }
        assert_eq!(EventType::from_code(15), None);
        assert_eq!(EventType::from_code(-1), None);
    }

    #[test]
    fn default_config_prioritises_signal_and_blacklists_noise() {
        let cfg = ActivityConfig::default();
        for ty in [EventType::Transfer, EventType::ContractEvent] {
            assert!(cfg.event_data_types.contains(&ty));
            assert!(cfg.involvement_types.contains(&ty));
            assert!(!cfg.event_type_blacklist.contains(&ty));
        }
        // The plumbing is dropped; the meaningful types are kept.
        assert!(cfg.event_type_blacklist.contains(&EventType::Guest));
        assert!(cfg.event_type_blacklist.contains(&EventType::Withhold));
        assert!(!cfg.event_type_blacklist.contains(&EventType::Execute));
        assert!(cfg.involvement_blacklist.is_empty());
    }

    #[test]
    fn flat_event_maps_to_event_type_and_has_no_contract() {
        let event = FlatEvent::Backrun(FlatEvtBackrun {
            sender: Addr::mock(1),
        });
        assert_eq!(EventType::from(&event), EventType::Backrun);
        assert!(event_contract(&event).is_none());
        assert!(contract_event_name(&event).is_none());
    }

    /// Applies the three migrations on an in-memory SQLite engine, then
    /// exercises the merged table: the address+attribute access paths (gateway
    /// / order_filled involving A), the empty-address marker for an address-less
    /// event, and the `event_data` split. Validates that the schema builds
    /// (composite PK with a bytea address incl. the empty value, the partial
    /// indexes, the side-table) end-to-end on a real engine.
    #[tokio::test]
    async fn schema_builds_and_merged_queries_round_trip() {
        use sea_orm::{ColumnTrait, Database, QueryFilter, QueryOrder};

        let db = Database::connect("sqlite::memory:").await.unwrap();
        let manager = sea_orm_migration::SchemaManager::new(&db);
        for migration in migrations::migrations() {
            migration.up(&manager).await.unwrap();
        }

        let a = vec![1u8; 20];
        let gateway = vec![0xAAu8; 20];
        let perps = vec![0xBBu8; 20];
        let tx = FlatCategory::Tx as i16;
        let ce = EventType::ContractEvent.code();

        events::Entity::insert_many([
            // gateway contract-event involving A.
            events::ActiveModel {
                address: Set(a.clone()),
                block_height: Set(100),
                category: Set(tx),
                category_index: Set(0),
                event_index: Set(4),
                event_type: Set(ce),
                contract: Set(Some(gateway.clone())),
                contract_event_name: Set(Some("bridge".to_string())),
            },
            // order_filled from perps involving A.
            events::ActiveModel {
                address: Set(a.clone()),
                block_height: Set(50),
                category: Set(tx),
                category_index: Set(0),
                event_index: Set(2),
                event_type: Set(ce),
                contract: Set(Some(perps.clone())),
                contract_event_name: Set(Some("order_filled".to_string())),
            },
            // an address-less event (empty-address marker) — a non-contract
            // event (here an Execute) with no participant: only contract events
            // carry `contract`, so it has none.
            events::ActiveModel {
                address: Set(Vec::new()),
                block_height: Set(70),
                category: Set(tx),
                category_index: Set(0),
                event_index: Set(0),
                event_type: Set(EventType::Execute.code()),
                contract: Set(None),
                contract_event_name: Set(None),
            },
        ])
        .exec_without_returning(&db)
        .await
        .unwrap();

        // "gateway events involving A" — the single block-100 row.
        let gw = events::Entity::find()
            .filter(events::Column::Address.eq(a.clone()))
            .filter(events::Column::Contract.eq(gateway.clone()))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(gw.len(), 1);
        assert_eq!(gw[0].block_height, 100);

        // "order_filled involving A".
        let of = events::Entity::find()
            .filter(events::Column::Address.eq(a.clone()))
            .filter(events::Column::ContractEventName.eq("order_filled".to_string()))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(of.len(), 1);
        assert_eq!(of[0].block_height, 50);

        // "all events involving A, newest-first" — gateway + order_filled = 2
        // (no sentinels now; the empty-address event does not involve A).
        let all = events::Entity::find()
            .filter(events::Column::Address.eq(a.clone()))
            .order_by_desc(events::Column::BlockHeight)
            .all(&db)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].block_height, 100);

        // The address-less event is stored under the empty address, distinct
        // from every real one.
        let mock = events::Entity::find()
            .filter(events::Column::Address.eq(Vec::<u8>::new()))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(mock.len(), 1);
        assert_eq!(mock[0].block_height, 70);

        // Data split: payload lives in the side-table, fetched by the shared
        // positional key.
        event_data::Entity::insert(event_data::ActiveModel {
            block_height: Set(100),
            category: Set(tx),
            category_index: Set(0),
            event_index: Set(4),
            data: Set(vec![1, 2, 3]),
        })
        .exec_without_returning(&db)
        .await
        .unwrap();
        let payload = event_data::Entity::find_by_id((100_i64, tx, 0_i32, 4_i32))
            .one(&db)
            .await
            .unwrap();
        assert_eq!(payload.unwrap().data, vec![1, 2, 3]);
    }
}
