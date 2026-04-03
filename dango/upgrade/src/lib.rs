use {
    dango_types::{
        Dimensionless, FundingPerUnit, FundingRate, Quantity, UsdPrice, UsdValue,
        perps::{self, RateSchedule},
    },
    grug::{
        Addr, BlockInfo, Duration, IndexedMap, Map, MultiIndex, Order as IterationOrder, StdResult,
        Storage, Timestamp, Uint128, UniqueIndex, addr,
    },
    grug_app::{AppResult, CONTRACT_NAMESPACE, CONTRACTS, StorageProvider},
    std::collections::{BTreeMap, VecDeque},
};

/// Address of the perps contract. Placeholder — fill in the actual value before
/// deploying the upgrade binary.
const PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Legacy types matching the pre-upgrade Borsh layout.
mod legacy {
    use super::*;

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    /// Read legacy user states using a plain Map (same namespace as the
    /// IndexedMap primary). Index entries are not affected by the value change.
    pub const USER_STATES: Map<Addr, UserState> = Map::new("us");

    pub const PAIR_STATES: Map<&perps::PairId, PairState> = Map::new("pair_state");

    pub const BIDS: Map<OrderKey, LimitOrder> = Map::new("bid");

    pub const ASKS: Map<OrderKey, LimitOrder> = Map::new("ask");

    pub const CONDITIONAL_ABOVE: IndexedMap<
        ConditionalOrderKey,
        ConditionalOrder,
        ConditionalOrderIndexes,
    > = IndexedMap::new(
        "conda",
        ConditionalOrderIndexes::new("conda", "conda__id", "conda__user"),
    );

    pub const CONDITIONAL_BELOW: IndexedMap<
        ConditionalOrderKey,
        ConditionalOrder,
        ConditionalOrderIndexes,
    > = IndexedMap::new(
        "condb",
        ConditionalOrderIndexes::new("condb", "condb__id", "condb__user"),
    );

    pub type OrderKey = (perps::PairId, UsdPrice, perps::OrderId);

    pub type ConditionalOrderKey = (perps::PairId, UsdPrice, perps::ConditionalOrderId);

    /// The Param struct before the upgrade, which contains the
    /// `max_conditional_orders` field.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct Param {
        pub max_unlocks: usize,
        pub max_open_orders: usize,
        pub max_conditional_orders: usize,
        pub maker_fee_rates: RateSchedule,
        pub taker_fee_rates: RateSchedule,
        pub protocol_fee_rate: Dimensionless,
        pub liquidation_fee_rate: Dimensionless,
        pub funding_period: Duration,
        pub vault_total_weight: Dimensionless,
        pub vault_cooldown_period: Duration,
        pub referral_active: bool,
        pub min_referrer_volume: UsdValue,
        pub referrer_commission_rates: RateSchedule,
    }

    /// The Position struct before the upgrade (3 fields, no conditional orders).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct Position {
        pub size: Quantity,
        pub entry_price: UsdPrice,
        pub entry_funding_per_unit: FundingPerUnit,
    }

    /// The UserState struct before the upgrade.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct UserState {
        pub margin: UsdValue,
        pub vault_shares: Uint128,
        pub positions: BTreeMap<perps::PairId, Position>,
        pub unlocks: VecDeque<perps::Unlock>,
        pub reserved_margin: UsdValue,
        pub open_order_count: usize,
        pub conditional_order_count: usize,
    }

    /// The PairState struct before the upgrade (no funding_rate field).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct PairState {
        pub long_oi: Quantity,
        pub short_oi: Quantity,
        pub funding_per_unit: FundingPerUnit,
    }

    /// The LimitOrder struct before the upgrade (no tp/sl child order fields).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct LimitOrder {
        pub user: Addr,
        pub size: Quantity,
        pub reduce_only: bool,
        pub reserved_margin: UsdValue,
        pub created_at: Timestamp,
    }

    /// The ConditionalOrder struct before the upgrade (stored in separate
    /// IndexedMaps, not embedded in Position).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct ConditionalOrder {
        pub user: Addr,
        pub size: Quantity,
        pub trigger_price: UsdPrice,
        pub trigger_direction: perps::TriggerDirection,
        pub max_slippage: Dimensionless,
        pub created_at: Timestamp,
    }

    #[grug::index_list(ConditionalOrderKey, ConditionalOrder)]
    pub struct ConditionalOrderIndexes<'a> {
        pub order_id:
            UniqueIndex<'a, ConditionalOrderKey, perps::ConditionalOrderId, ConditionalOrder>,
        pub user: MultiIndex<'a, ConditionalOrderKey, Addr, ConditionalOrder>,
    }

    impl ConditionalOrderIndexes<'static> {
        pub const fn new(
            pk_namespace: &'static str,
            order_id_namespace: &'static str,
            user_namespace: &'static str,
        ) -> Self {
            ConditionalOrderIndexes {
                order_id: UniqueIndex::new(
                    |(_, _, order_id), _| *order_id,
                    pk_namespace,
                    order_id_namespace,
                ),
                user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
            }
        }
    }
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // This upgrade only applies to the testnet. The perps contract isn't deployed
    // on mainnet, so nothing to do.
    // Check whether perps contract exists. If not, skip.
    if !CONTRACTS.has(&storage, PERPS_ADDRESS) {
        tracing::info!("Perps contract not found. Nothing to do");

        return Ok(());
    }

    let mut perps_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, PERPS_ADDRESS.as_ref()]);

    Ok(_do_upgrade(&mut perps_storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    // 1. Migrate Param: load the old layout, convert to new (dropping
    //    max_conditional_orders), and save.

    {
        let old_param = legacy::PARAM.load(storage)?;

        let new_param = perps::Param {
            max_unlocks: old_param.max_unlocks,
            max_open_orders: old_param.max_open_orders,
            maker_fee_rates: old_param.maker_fee_rates,
            taker_fee_rates: old_param.taker_fee_rates,
            protocol_fee_rate: old_param.protocol_fee_rate,
            liquidation_fee_rate: old_param.liquidation_fee_rate,
            funding_period: old_param.funding_period,
            vault_total_weight: old_param.vault_total_weight,
            vault_cooldown_period: old_param.vault_cooldown_period,
            referral_active: old_param.referral_active,
            min_referrer_volume: old_param.min_referrer_volume,
            referrer_commission_rates: old_param.referrer_commission_rates,
        };

        dango_perps::state::PARAM.save(storage, &new_param)?;

        tracing::info!("Migrated Param (removed max_conditional_orders)");
    }

    // -------------------------------------------------------------------------

    // 2. Wipe old CONDITIONAL_ABOVE/BELOW IndexedMaps (primary + indexes).

    {
        legacy::CONDITIONAL_ABOVE.clear_all(storage);
        legacy::CONDITIONAL_BELOW.clear_all(storage);

        tracing::info!("Wiped all conditional orders");
    }

    // -------------------------------------------------------------------------

    // 3. Migrate UserState records: read with legacy layout, convert to new
    //    layout (drop conditional_order_count, add conditional_order fields to
    //    Position).

    {
        let all_users = legacy::USER_STATES
            .range(storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        let new_user_states: Map<Addr, perps::UserState> = Map::new("us");

        for (addr, old_us) in all_users {
            let new_positions = old_us
                .positions
                .into_iter()
                .map(|(pair_id, old_pos)| {
                    (pair_id, perps::Position {
                        size: old_pos.size,
                        entry_price: old_pos.entry_price,
                        entry_funding_per_unit: old_pos.entry_funding_per_unit,
                        conditional_order_above: None,
                        conditional_order_below: None,
                    })
                })
                .collect();

            let new_us = perps::UserState {
                margin: old_us.margin,
                vault_shares: old_us.vault_shares,
                positions: new_positions,
                unlocks: old_us.unlocks,
                reserved_margin: old_us.reserved_margin,
                open_order_count: old_us.open_order_count,
            };

            new_user_states.save(storage, addr, &new_us)?;
        }

        tracing::info!("Migrated UserState records");
    }

    // -------------------------------------------------------------------------

    // 4. Migrate PairState records: load with legacy layout (no funding_rate),
    //    convert to new layout (funding_rate defaults to zero).

    {
        let all_pairs = legacy::PAIR_STATES
            .range(storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for (pair_id, old_ps) in &all_pairs {
            let new_ps = perps::PairState {
                long_oi: old_ps.long_oi,
                short_oi: old_ps.short_oi,
                funding_per_unit: old_ps.funding_per_unit,
                funding_rate: FundingRate::ZERO,
            };

            dango_perps::state::PAIR_STATES.save(storage, pair_id, &new_ps)?;
        }

        tracing::info!("Migrated {} PairState records", all_pairs.len());
    }

    // -------------------------------------------------------------------------

    // 5. Migrate LimitOrder records in BIDS and ASKS: add tp/sl = None.
    //    Indexes (bid__id, bid__user, ask__id, ask__user) remain valid because
    //    the key and indexed fields (order_id, user) are unchanged.

    {
        let new_bids: Map<legacy::OrderKey, perps::LimitOrder> = Map::new("bid");
        let new_asks: Map<legacy::OrderKey, perps::LimitOrder> = Map::new("ask");

        for (old_map, new_map, label) in [
            (&legacy::BIDS, &new_bids, "bid"),
            (&legacy::ASKS, &new_asks, "ask"),
        ] {
            let all = old_map
                .range(storage, None, None, IterationOrder::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            let count = all.len();

            for (key, old_order) in all {
                new_map.save(storage, key, &perps::LimitOrder {
                    user: old_order.user,
                    size: old_order.size,
                    reduce_only: old_order.reduce_only,
                    reserved_margin: old_order.reserved_margin,
                    created_at: old_order.created_at,
                    tp: None,
                    sl: None,
                })?;
            }

            tracing::info!("Migrated {count} {label} orders");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_perps::state::{
            ASKS, BIDS, COMMISSION_RATE_OVERRIDES, DEPTHS, FEE_SHARE_RATIO, LONGS, NEXT_ORDER_ID,
            PAIR_IDS, PAIR_PARAMS, PAIR_STATES, REFEREE_TO_REFERRER,
            REFERRER_TO_REFEREE_STATISTICS, SHORTS, STATE, USER_REFERRAL_DATA, USER_STATES,
            VOLUMES,
        },
        grug::{MockStorage, Order as IterationOrder, StdResult, Storage},
        std::collections::BTreeMap,
    };

    /// Load the testnet perps storage fixture and return a MockStorage populated
    /// with its key-value pairs.
    fn load_fixture() -> MockStorage {
        let bytes = include_bytes!("../testdata/perps_storage.borsh");
        let fixture: BTreeMap<Vec<u8>, Vec<u8>> =
            borsh::from_slice(bytes).expect("failed to deserialize fixture");

        let mut storage = MockStorage::new();
        for (k, v) in &fixture {
            storage.write(k, v);
        }
        storage
    }

    #[test]
    #[ignore = "requires testdata/perps_storage.borsh generated by dump_perps_storage script"]
    fn upgrade_migrates_testnet_storage() {
        let mut storage = load_fixture();

        _do_upgrade(&mut storage).unwrap();

        // --- Items ---

        dango_perps::state::PARAM.load(&storage).unwrap();
        STATE.load(&storage).unwrap();
        NEXT_ORDER_ID.load(&storage).unwrap();
        dango_perps::state::LAST_VAULT_ORDERS_UPDATE
            .load(&storage)
            .unwrap();
        PAIR_IDS.load(&storage).unwrap();

        // --- Maps (iterate all, every entry must deserialize) ---

        let pair_params: Vec<_> = PAIR_PARAMS
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert!(!pair_params.is_empty());

        let pair_states: Vec<_> = PAIR_STATES
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert!(!pair_states.is_empty());

        USER_STATES
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        BIDS.range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        ASKS.range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        DEPTHS
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        VOLUMES
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        LONGS
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        SHORTS
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        // --- Referral maps ---

        REFEREE_TO_REFERRER
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        FEE_SHARE_RATIO
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        COMMISSION_RATE_OVERRIDES
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        USER_REFERRAL_DATA
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        REFERRER_TO_REFEREE_STATISTICS
            .range(&storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        // --- Deleted data must be gone ---

        assert!(legacy::CONDITIONAL_ABOVE.is_empty(&storage));
        assert!(legacy::CONDITIONAL_BELOW.is_empty(&storage));
    }
}
