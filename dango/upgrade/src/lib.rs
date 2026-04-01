use {
    dango_types::{
        Dimensionless, UsdValue,
        perps::{self, RateSchedule},
    },
    grug::{Addr, BlockInfo, Duration, Map, Order as IterationOrder, StdResult, Storage, Uint128},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::{BTreeMap, VecDeque},
};

/// Address of the perps contract. Placeholder — fill in the actual value before
/// deploying the upgrade binary.
const PERPS_ADDRESS: Addr = Addr::ZERO;

/// Legacy types matching the pre-upgrade Borsh layout.
mod legacy {
    use super::*;

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

    /// The UserState struct before the upgrade, which contains the
    /// `conditional_order_count` field as its last field.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct UserState {
        pub margin: UsdValue,
        pub vault_shares: Uint128,
        pub positions: BTreeMap<perps::PairId, perps::Position>,
        pub unlocks: VecDeque<perps::Unlock>,
        pub reserved_margin: UsdValue,
        pub open_order_count: usize,
        pub conditional_order_count: usize,
    }

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    /// Read legacy user states using a plain Map (same namespace as the
    /// IndexedMap primary). Index entries are not affected by the value change.
    pub const USER_STATES: Map<Addr, UserState> = Map::new("us");
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let mut perps_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, PERPS_ADDRESS.as_ref()]);

    // 1. Migrate Param: load the old layout, convert to new (dropping
    //    max_conditional_orders), and save.
    let old_param = legacy::PARAM.load(&perps_storage)?;

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

    dango_perps::PARAM.save(&mut perps_storage, &new_param)?;

    tracing::info!("Migrated Param (removed max_conditional_orders)");

    // 2. Cancel all existing TP/SL orders by wiping the CONDITIONAL_ABOVE and
    //    CONDITIONAL_BELOW indexed maps (primary + all indexes).
    dango_perps::CONDITIONAL_ABOVE.clear_all(&mut perps_storage);
    dango_perps::CONDITIONAL_BELOW.clear_all(&mut perps_storage);

    tracing::info!("Wiped all conditional orders");

    // 3. Migrate UserState records: read with legacy layout (6 fields including
    //    conditional_order_count), convert to new layout (5 fields), save back.
    //
    //    Using Map<Addr, T> with namespace "us" reads/writes the same primary
    //    entries as the IndexedMap. The index (earliest_unlock_end_time) is
    //    computed from the unlocks field which is unchanged, so index entries
    //    remain valid.
    let all_users = legacy::USER_STATES
        .range(&perps_storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let new_user_states: Map<Addr, perps::UserState> = Map::new("us");

    for (addr, old_us) in all_users {
        let new_us = perps::UserState {
            margin: old_us.margin,
            vault_shares: old_us.vault_shares,
            positions: old_us.positions,
            unlocks: old_us.unlocks,
            reserved_margin: old_us.reserved_margin,
            open_order_count: old_us.open_order_count,
        };
        new_user_states.save(&mut perps_storage, addr, &new_us)?;
    }

    tracing::info!("Migrated UserState records (removed conditional_order_count)");

    Ok(())
}
