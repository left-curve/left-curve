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
    crate::{Ctx, Projection, WhiteOrBlackList},
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
    /// Which event types are **kept** (indexed at all). An event whose type the
    /// filter rejects gets no `events` row and no payload. Default: a blacklist
    /// of the address-less system noise (guest/withhold/authenticate/finalize/…)
    /// that dominates the raw stream and is not user-facing activity.
    pub event_type_filter: WhiteOrBlackList<EventType>,
    /// Which event types have their payload (`zstd(borsh(event))`) stored in
    /// `event_data`; others are hydrated from the raw block on demand. Default:
    /// a whitelist of the priority types.
    pub event_data_filter: WhiteOrBlackList<EventType>,
    /// Which event types have their participants extracted into the `events`
    /// rows (one row per participant); other kept types get a single
    /// empty-address row. Default: a whitelist of the priority types.
    pub involvement_filter: WhiteOrBlackList<EventType>,
    /// Addresses excluded from participation at write time — the deployment's
    /// system contracts, merged in by the cli from the node's `app_config`.
    pub involvement_blacklist: HashSet<Addr>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        let priority = HashSet::from([EventType::Transfer, EventType::ContractEvent]);
        Self {
            // The pure plumbing: emitted on every call/tx, never carries a
            // meaningful participant. Measured ~81% of the raw mainnet stream.
            event_type_filter: WhiteOrBlackList::Blacklist(HashSet::from([
                EventType::Guest,
                EventType::Withhold,
                EventType::Authenticate,
                EventType::Finalize,
                EventType::Backrun,
                EventType::Reply,
            ])),
            event_data_filter: WhiteOrBlackList::Whitelist(priority.clone()),
            involvement_filter: WhiteOrBlackList::Whitelist(priority),
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
                timestamp: Set(timestamp),
            });

            // A cronjob is a single commitment group; `flatten_unit` numbers
            // its events exactly as the read path will (one source of truth).
            // Cron has no sender, so no `transactions.sender` row feeds query
            // 1's sender side — only its participants.
            let flat = flatten_unit(block, FlatCategory::Cron as i16, cron_idx);
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
                timestamp: Set(timestamp),
            });

            // `flatten_unit` flattens the four commitment groups (withhold,
            // authenticate, msgs + backrun, finalize) on a fresh per-tx id —
            // the same numbering the read path reuses.
            let flat = flatten_unit(block, FlatCategory::Tx as i16, tx_idx);
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
        for (position, info) in flat.iter().enumerate() {
            // The read path (`Event::data`'s non-priority fallback) re-flattens
            // the unit and finds an event by its `event_index`; the write path
            // stores that same `id.event_index`. The numbering is dense `0..n`
            // matching the flattened Vec position — pin that here so a flatten
            // change can't silently desync the two sides.
            debug_assert_eq!(
                info.id.event_index as usize, position,
                "flattened event_index must equal its position within the unit"
            );
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

        if !self.config.event_type_filter.allows(&event_type) {
            return Ok(());
        }

        let category = info.id.category as i16;
        let category_index = info.id.category_index as i32;
        let event_index = info.id.event_index as i32;

        let contract = event_contract(event).map(|a| a.as_ref().to_vec());
        let contract_event_name = contract_event_name(event);

        // Payload into the side-table, only for priority types.
        if self.config.event_data_filter.allows(&event_type) {
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
        let mut addresses: Vec<Vec<u8>> = if self.config.involvement_filter.allows(&event_type) {
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

/// Inverse of [`compress_event`]: the stored `zstd(borsh(FlatEvent))` back to
/// the event. The read path uses it to hydrate a priority event's payload from
/// the `event_data` blob the feed join carried.
pub(crate) fn decompress_event(data: &[u8]) -> AnyResult<FlatEvent> {
    let borshed = zstd::decode_all(data)?;
    Ok(borsh::from_slice(&borshed)?)
}

/// Flatten a single unit of a block — the cronjob or transaction at
/// `(category, category_index)` — into its positioned events, numbered exactly
/// as [`process`](ActivityProjection::process) writes them. Shared by the write
/// path (staging the `events` rows) and the read path (hydrating a non-priority
/// event's payload, absent from `event_data`), so `event_index` means the same
/// on both sides. Empty when the index is out of range or the category is
/// neither cron nor tx.
pub(crate) fn flatten_unit(
    block: &BlockData,
    category: i16,
    category_index: usize,
) -> Vec<FlatEventInfo> {
    let block_height = block.block.info.height;
    if category == FlatCategory::Cron as i16 {
        let Some(cron) = block.outcome.cron_outcomes.get(category_index) else {
            return Vec::new();
        };
        let mut event_id = EventId::new(block_height, FlatCategory::Cron, category_index as u32, 0);
        flatten_commitment_status(&mut event_id, cron.cron_event.clone())
    } else if category == FlatCategory::Tx as i16 {
        let Some(tx) = block.outcome.tx_outcomes.get(category_index) else {
            return Vec::new();
        };
        flatten_tx_events(tx.events.clone(), block_height, category_index as u32)
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_primitives::{FlatEvtBackrun, Hash256},
    };

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
            assert!(cfg.event_data_filter.allows(&ty));
            assert!(cfg.involvement_filter.allows(&ty));
            assert!(cfg.event_type_filter.allows(&ty));
        }
        // The plumbing is dropped; the meaningful types are kept.
        assert!(!cfg.event_type_filter.allows(&EventType::Guest));
        assert!(!cfg.event_type_filter.allows(&EventType::Withhold));
        assert!(cfg.event_type_filter.allows(&EventType::Execute));
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

    // ---- Postgres feed integration test ----
    //
    // The eight feeds are **hand-written** SQL run through a `Statement`
    // tagged `DbBackend::Postgres` — `DISTINCT ON`, a `WITH … UNION` of two
    // arms each carrying their own `ORDER BY … LIMIT`, row-comparison keysets,
    // and a correlated `activity_event_data` sub-select. None of that executes
    // in the SQLite round-trip test above (which drives the *typed* sea-orm
    // builder), and the SDL test never runs a resolver — so a SQL-shape bug
    // (a wrong table name, an unparenthesised `UNION` arm) compiles clean and
    // only fails at runtime against Postgres. This test closes that gap: it
    // runs **every feed** against a real Postgres and asserts the rows, so the
    // class of bug that needs a live engine to surface is caught in CI.
    //
    // Skipped (not failed) when no Postgres is reachable, so `cargo test` stays
    // green on a bare machine; CI's backend job always provides one.

    /// One `activity_transactions` row.
    fn tx_row(
        height: i64,
        idx: i32,
        kind: i16,
        hash: Option<Vec<u8>>,
        sender: Option<Vec<u8>>,
        success: bool,
    ) -> transactions::ActiveModel {
        transactions::ActiveModel {
            block_height: Set(height),
            idx: Set(idx),
            kind: Set(kind),
            hash: Set(hash),
            sender: Set(sender),
            success: Set(success),
            timestamp: Set(0),
        }
    }

    /// One `activity_events` row (an event × one participant).
    #[allow(clippy::too_many_arguments)]
    fn ev_row(
        address: Vec<u8>,
        height: i64,
        category: i16,
        category_index: i32,
        event_index: i32,
        event_type: i16,
        contract: Option<Vec<u8>>,
        name: Option<&str>,
    ) -> events::ActiveModel {
        events::ActiveModel {
            address: Set(address),
            block_height: Set(height),
            category: Set(category),
            category_index: Set(category_index),
            event_index: Set(event_index),
            event_type: Set(event_type),
            contract: Set(contract),
            contract_event_name: Set(name.map(str::to_string)),
        }
    }

    /// Connect to the `DATABASE_URL` Postgres (default the repo's local
    /// `grug_test`), carve out a throwaway schema for this test, and return a
    /// handle whose `search_path` is scoped to it. `None` — skip — when no
    /// Postgres is reachable.
    async fn pg_test_schema() -> Option<(sea_orm::DatabaseConnection, String)> {
        use sea_orm::{ConnectOptions, ConnectionTrait, Database};

        let base_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/grug_test".to_string());
        let schema = format!("hist_activity_it_{}", uuid::Uuid::new_v4().simple());

        let mut opt = ConnectOptions::new(base_url);
        // `set_schema_search_path` runs `SET search_path = "schema"` on every
        // pooled connection (sea-orm's `after_connect`); pin the pool to one
        // connection so the create-schema / migrate / query / drop sequence is
        // deterministic. Short timeouts so the skip path (no Postgres) bails in
        // a couple of seconds instead of blocking on the default 30 s acquire.
        opt.set_schema_search_path(schema.clone())
            .max_connections(1)
            .connect_timeout(std::time::Duration::from_secs(2))
            .acquire_timeout(std::time::Duration::from_secs(2))
            .sqlx_logging(false);

        let db = match Database::connect(opt).await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("skipping Postgres feed test — no reachable DATABASE_URL ({err})");
                return None;
            },
        };
        // The search_path already points at `schema`; create it before the
        // migrations land any unqualified table in a missing namespace.
        db.execute_unprepared(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\""))
            .await
            .expect("create throwaway schema");
        Some((db, schema))
    }

    type ReadSchema = async_graphql::Schema<
        ActivityQuery,
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    >;

    /// Execute a query, assert it produced no errors (a SQL-shape bug surfaces
    /// here), and return the JSON `data`.
    async fn exec(schema: &ReadSchema, query: String) -> serde_json::Value {
        let resp = schema.execute(query.as_str()).await;
        assert!(
            resp.errors.is_empty(),
            "unexpected GraphQL errors for `{query}`:\n{:#?}",
            resp.errors
        );
        resp.data.into_json().expect("response data to json")
    }

    /// The `blockHeight`s of a connection feed's nodes, in order.
    fn conn_heights(value: &serde_json::Value, field: &str) -> Vec<u64> {
        value[field]["edges"]
            .as_array()
            .expect("edges array")
            .iter()
            .map(|edge| edge["node"]["blockHeight"].as_u64().expect("blockHeight"))
            .collect()
    }

    /// The `blockHeight`s of a plain-list feed's elements, in order.
    fn list_heights(value: &serde_json::Value, field: &str) -> Vec<u64> {
        value[field]
            .as_array()
            .expect("list")
            .iter()
            .map(|node| node["blockHeight"].as_u64().expect("blockHeight"))
            .collect()
    }

    #[tokio::test]
    async fn feeds_execute_against_postgres() {
        use {
            async_graphql::{EmptyMutation, EmptySubscription, Schema},
            sea_orm::ConnectionTrait,
        };

        let Some((db, schema_name)) = pg_test_schema().await else {
            return;
        };

        // Apply the three migrations (tables + partial / fillfactor indexes)
        // into the throwaway schema.
        let manager = sea_orm_migration::SchemaManager::new(&db);
        for migration in migrations::migrations() {
            migration.up(&manager).await.expect("migration up");
        }

        // ---- fixture: addresses, hashes, codes ----
        let a = Addr::try_from(vec![0x0Au8; 20]).unwrap(); // the subject address
        let b = Addr::try_from(vec![0x0Bu8; 20]).unwrap(); // another sender
        let c = Addr::try_from(vec![0x0Cu8; 20]).unwrap(); // a co-participant
        let gateway = Addr::try_from(vec![0xAAu8; 20]).unwrap();
        let perps = Addr::try_from(vec![0xBBu8; 20]).unwrap();
        let h1 = Hash256::try_from(vec![0x11u8; 32]).unwrap(); // shared by two units
        let h2 = Hash256::try_from(vec![0x22u8; 32]).unwrap();
        let hx = Hash256::try_from(vec![0xFFu8; 32]).unwrap(); // matches nothing

        let tx = FlatCategory::Tx as i16;
        let cron = FlatCategory::Cron as i16;
        let ce = EventType::ContractEvent.code();
        let transfer = EventType::Transfer.code();

        // ---- seed units ----
        // #1 h100 tx: A is the SENDER.
        // #2 h90  tx: B sent it; A only PARTICIPATES (via an event).
        // #3 h80  cron: A participates; cron has no sender.
        // #4 h70  tx: same hash as #1 but a different block & sender — A is
        //         neither sender nor participant, so only `transactionsByHash`
        //         (not `transactionsInvolving(A)`) sees it.
        transactions::Entity::insert_many([
            tx_row(
                100,
                0,
                tx,
                Some(h1.as_ref().to_vec()),
                Some(a.as_ref().to_vec()),
                true,
            ),
            tx_row(
                90,
                0,
                tx,
                Some(h2.as_ref().to_vec()),
                Some(b.as_ref().to_vec()),
                true,
            ),
            tx_row(80, 0, cron, None, None, true),
            tx_row(
                70,
                0,
                tx,
                Some(h1.as_ref().to_vec()),
                Some(b.as_ref().to_vec()),
                false,
            ),
        ])
        .exec_without_returning(&db)
        .await
        .unwrap();

        // ---- seed events (one row per event × participant) ----
        events::Entity::insert_many([
            // gateway "bridge" in unit #2, participant A (h90, eidx 2).
            ev_row(
                a.as_ref().to_vec(),
                90,
                tx,
                0,
                2,
                ce,
                Some(gateway.as_ref().to_vec()),
                Some("bridge"),
            ),
            // perps "order_filled" in unit #1, participants B and C (NOT A) —
            // two rows the DISTINCT-ON feeds must collapse to one event.
            ev_row(
                b.as_ref().to_vec(),
                100,
                tx,
                0,
                5,
                ce,
                Some(perps.as_ref().to_vec()),
                Some("order_filled"),
            ),
            ev_row(
                c.as_ref().to_vec(),
                100,
                tx,
                0,
                5,
                ce,
                Some(perps.as_ref().to_vec()),
                Some("order_filled"),
            ),
            // a Transfer in the cron unit #3, participant A (h80, eidx 0).
            ev_row(a.as_ref().to_vec(), 80, cron, 0, 0, transfer, None, None),
        ])
        .exec_without_returning(&db)
        .await
        .unwrap();

        // ---- seed the priority payload for the gateway event ----
        let payload = FlatEvent::Backrun(FlatEvtBackrun { sender: a });
        event_data::Entity::insert(event_data::ActiveModel {
            block_height: Set(90),
            category: Set(tx),
            category_index: Set(0),
            event_index: Set(2),
            data: Set(compress_event(&payload).unwrap()),
        })
        .exec_without_returning(&db)
        .await
        .unwrap();

        // Build the read schema over the live Postgres connection. No
        // `BlockLoader` is registered: every query below selects scalar columns
        // or the *priority* `data` (decompressed from the joined blob, no block
        // load), so no resolver ever reaches for the loader.
        let schema = Schema::build(ActivityQuery, EmptyMutation, EmptySubscription)
            .data(db.clone())
            .finish();

        let a_hex = a.to_string();
        let b_hex = b.to_string();
        let gw_hex = gateway.to_string();
        let perps_hex = perps.to_string();
        let h1_hex = h1.to_string();
        let h2_hex = h2.to_string();
        let hx_hex = hx.to_string();
        let subst = |query: &str| {
            query
                .replace("$A", &a_hex)
                .replace("$B", &b_hex)
                .replace("$GW", &gw_hex)
                .replace("$PERPS", &perps_hex)
                .replace("$H1", &h1_hex)
                .replace("$H2", &h2_hex)
                .replace("$HX", &hx_hex)
        };

        // ===== query 1: transactionsInvolving (the involved ∪ sender UNION) =====

        // Default (role omitted): the union of A-as-sender (#1) and
        // A-as-participant (#2 tx, #3 cron), newest-first, deduped. This is the
        // exact path the unparenthesised-`UNION` bug broke at runtime.
        let v = exec(
            &schema,
            subst(
                "{ transactionsInvolving(address:\"$A\") { edges { node { blockHeight kind } } } }",
            ),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![100, 90, 80]);
        // The h80 unit is the cronjob.
        assert_eq!(
            v["transactionsInvolving"]["edges"][2]["node"]["kind"],
            serde_json::json!("CRON")
        );

        // role: SENDER → only the sender side (#1).
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", role: SENDER) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![100]);

        // role: PARTICIPANT → only the involved side (#2, #3).
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", role: PARTICIPANT) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![90, 80]);

        // kind: CRON → the sender side is dropped (cron has no sender); only the
        // cron unit #3.
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", kind: CRON) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![80]);

        // kind: TRANSACTION → sender side (#1) + the tx participant side (#2).
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", kind: TRANSACTION) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![100, 90]);

        // role: SENDER + kind: CRON → no side can match; an empty page, no query.
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", role: SENDER, kind: CRON) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), Vec::<u64>::new());

        // Keyset pagination across the UNION: page the default feed 2 + 1.
        let v = exec(
            &schema,
            subst("{ transactionsInvolving(address:\"$A\", first: 2) { edges { node { blockHeight } } pageInfo { hasNextPage endCursor } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![100, 90]);
        assert_eq!(
            v["transactionsInvolving"]["pageInfo"]["hasNextPage"],
            serde_json::json!(true)
        );
        let cursor = v["transactionsInvolving"]["pageInfo"]["endCursor"]
            .as_str()
            .expect("endCursor")
            .to_string();
        let v = exec(
            &schema,
            subst(&format!(
                "{{ transactionsInvolving(address:\"$A\", first: 2, after:\"{cursor}\") {{ edges {{ node {{ blockHeight }} }} pageInfo {{ hasNextPage }} }} }}"
            )),
        )
        .await;
        assert_eq!(conn_heights(&v, "transactionsInvolving"), vec![80]);
        assert_eq!(
            v["transactionsInvolving"]["pageInfo"]["hasNextPage"],
            serde_json::json!(false)
        );

        // ===== transactionsByHash (un-paginated; hash is not unique) =====

        // h1 maps to two units (#1 and #4), newest-first.
        let v = exec(
            &schema,
            subst("{ transactionsByHash(hash:\"$H1\") { blockHeight } }"),
        )
        .await;
        assert_eq!(list_heights(&v, "transactionsByHash"), vec![100, 70]);
        // h2 maps to one.
        let v = exec(
            &schema,
            subst("{ transactionsByHash(hash:\"$H2\") { blockHeight } }"),
        )
        .await;
        assert_eq!(list_heights(&v, "transactionsByHash"), vec![90]);
        // an unknown hash maps to none.
        let v = exec(
            &schema,
            subst("{ transactionsByHash(hash:\"$HX\") { blockHeight } }"),
        )
        .await;
        assert_eq!(list_heights(&v, "transactionsByHash"), Vec::<u64>::new());

        // ===== event feeds =====

        // by type — DISTINCT ON collapses the two order_filled participant rows
        // into one event; bridge + order_filled, newest-first.
        let v = exec(
            &schema,
            "{ eventsByType(type: CONTRACT_EVENT) { edges { node { blockHeight } } } }".to_string(),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsByType"), vec![100, 90]);

        let v = exec(
            &schema,
            "{ eventsByType(type: TRANSFER) { edges { node { blockHeight } } } }".to_string(),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsByType"), vec![80]);

        // involving an address (one row per event for A) — bridge + cron transfer.
        let v = exec(
            &schema,
            subst("{ eventsInvolving(address:\"$A\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsInvolving"), vec![90, 80]);

        let v = exec(
            &schema,
            subst("{ eventsInvolving(address:\"$A\", type: TRANSFER) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsInvolving"), vec![80]);

        // by contract (DISTINCT ON), with and without a name filter.
        let v = exec(
            &schema,
            subst("{ contractEvents(contract:\"$GW\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEvents"), vec![90]);
        let v = exec(
            &schema,
            subst("{ contractEvents(contract:\"$PERPS\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEvents"), vec![100]);
        let v = exec(
            &schema,
            subst("{ contractEvents(contract:\"$PERPS\", names:[\"order_filled\"]) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEvents"), vec![100]);
        let v = exec(
            &schema,
            subst("{ contractEvents(contract:\"$PERPS\", names:[\"does_not_exist\"]) { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEvents"), Vec::<u64>::new());

        // contract events involving an address.
        let v = exec(
            &schema,
            subst("{ contractEventsInvolving(address:\"$A\", contract:\"$GW\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEventsInvolving"), vec![90]);
        let v = exec(
            &schema,
            subst("{ contractEventsInvolving(address:\"$B\", contract:\"$PERPS\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEventsInvolving"), vec![100]);
        // A did not participate in the perps event (only B and C).
        let v = exec(
            &schema,
            subst("{ contractEventsInvolving(address:\"$A\", contract:\"$PERPS\") { edges { node { blockHeight } } } }"),
        )
        .await;
        assert_eq!(
            conn_heights(&v, "contractEventsInvolving"),
            Vec::<u64>::new()
        );

        // ===== event payload (the look-ahead + correlated event_data join) =====

        // Selecting `data` drives `wants_event_data` true → the feed pulls the
        // priority blob through the correlated `activity_event_data` sub-select
        // → it decompresses to the stored event. Round-trips the payload.
        let v = exec(
            &schema,
            subst("{ contractEvents(contract:\"$GW\") { edges { node { blockHeight data } } } }"),
        )
        .await;
        assert_eq!(conn_heights(&v, "contractEvents"), vec![90]);
        assert_eq!(
            v["contractEvents"]["edges"][0]["node"]["data"],
            serde_json::to_value(&payload).unwrap(),
            "the priority payload should round-trip through the event_data join"
        );

        // ===== event-feed keyset pagination (the row-comparison cursor) =====

        let v = exec(
            &schema,
            "{ eventsByType(type: CONTRACT_EVENT, first: 1) { edges { node { blockHeight } } pageInfo { hasNextPage endCursor } } }".to_string(),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsByType"), vec![100]);
        assert_eq!(
            v["eventsByType"]["pageInfo"]["hasNextPage"],
            serde_json::json!(true)
        );
        let cursor = v["eventsByType"]["pageInfo"]["endCursor"]
            .as_str()
            .expect("endCursor")
            .to_string();
        let v = exec(
            &schema,
            format!(
                "{{ eventsByType(type: CONTRACT_EVENT, first: 1, after:\"{cursor}\") {{ edges {{ node {{ blockHeight }} }} pageInfo {{ hasNextPage }} }} }}"
            ),
        )
        .await;
        assert_eq!(conn_heights(&v, "eventsByType"), vec![90]);
        assert_eq!(
            v["eventsByType"]["pageInfo"]["hasNextPage"],
            serde_json::json!(false)
        );

        // ---- teardown: drop the throwaway schema ----
        db.execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE"))
            .await
            .ok();
    }
}
