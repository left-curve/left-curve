use {
    dango_types::{
        Dimensionless, FundingPerUnit, FundingRate, Quantity, UsdPrice, UsdValue,
        perps::{self, RateSchedule},
    },
    grug::{
        Addr, BlockInfo, Duration, Map, Order as IterationOrder, StdResult, Storage, Uint128, addr,
        increment_last_byte,
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

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    /// Read legacy user states using a plain Map (same namespace as the
    /// IndexedMap primary). Index entries are not affected by the value change.
    pub const USER_STATES: Map<Addr, UserState> = Map::new("us");

    pub const PAIR_STATES: Map<&perps::PairId, PairState> = Map::new("pair_state");
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

    // -------------------------------------------------------------------------

    // 1. Migrate Param: load the old layout, convert to new (dropping
    //    max_conditional_orders), and save.

    {
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

        dango_perps::state::PARAM.save(&mut perps_storage, &new_param)?;

        tracing::info!("Migrated Param (removed max_conditional_orders)");
    }

    // -------------------------------------------------------------------------

    // 2. Wipe old CONDITIONAL_ABOVE/BELOW maps via raw remove_range.
    //    These maps no longer exist as IndexedMap constants, so we clear them
    //    by their storage namespace prefixes.
    //    Namespaces: "conda", "conda__id", "conda__user", "condb", "condb__id", "condb__user"

    {
        for ns in &[
            b"conda" as &[u8],
            b"conda__id",
            b"conda__user",
            b"condb",
            b"condb__id",
            b"condb__user",
        ] {
            let max = increment_last_byte(ns.to_vec());
            perps_storage.remove_range(Some(ns), Some(&max));
        }

        tracing::info!("Wiped all conditional orders");
    }

    // -------------------------------------------------------------------------

    // 3. Migrate UserState records: read with legacy layout, convert to new
    //    layout (drop conditional_order_count, add conditional_order fields to
    //    Position).

    {
        let all_users = legacy::USER_STATES
            .range(&perps_storage, None, None, IterationOrder::Ascending)
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

            new_user_states.save(&mut perps_storage, addr, &new_us)?;
        }

        tracing::info!("Migrated UserState records");
    }

    // -------------------------------------------------------------------------

    // 4. Migrate PairState records: load with legacy layout (no funding_rate),
    //    convert to new layout (funding_rate defaults to zero).

    {
        let all_pairs = legacy::PAIR_STATES
            .range(&perps_storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for (pair_id, old_ps) in &all_pairs {
            let new_ps = perps::PairState {
                long_oi: old_ps.long_oi,
                short_oi: old_ps.short_oi,
                funding_per_unit: old_ps.funding_per_unit,
                funding_rate: FundingRate::ZERO,
            };

            dango_perps::state::PAIR_STATES.save(&mut perps_storage, pair_id, &new_ps)?;
        }

        tracing::info!("Migrated {} PairState records", all_pairs.len());
    }

    Ok(())
}
