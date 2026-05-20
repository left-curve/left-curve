use {
    anyhow::Result,
    dango_order_book::{PairId, Quantity},
    dango_perps::{core::update_lot_size, state::PAIR_PARAMS},
    dango_types::perps::PairParam,
    grug::{Addr, BlockInfo, Dec128_6, Order, StdError, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    std::str::FromStr,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Find the address of the perps contract corresponding to the current chain.
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    // Run perps-contract-scoped migrations under the contract's storage prefix.
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);
    upgrade_lot_size(&mut perps_storage).map_err(|e| StdError::host(e.to_string()))?;

    Ok(())
}

/// Lift legacy `PairParam` rows onto the new shape (carries a `lot_size`
/// field that pre-upgrade chain bytes don't have) and re-align resting
/// orders and open positions to the new lot grid via
/// `dango_perps::core::update_lot_size`.
///
/// Hardcodes the per-pair `lot_size` to match the genesis configuration:
/// `perp/btcusd` = 0.00001, `perp/ethusd` = 0.0001. Pairs not in this map
/// are migrated with `lot_size = 0` (constraint disabled), which can be
/// raised later via a separate upgrade.
fn upgrade_lot_size(storage: &mut dyn Storage) -> Result<()> {
    let entries = legacy::PAIR_PARAMS
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (pair_id, old) in entries {
        let new_lot_size = lot_size_for(&pair_id);
        let new = PairParam {
            tick_size: old.tick_size,
            min_order_value: old.min_order_value,
            lot_size: new_lot_size,
            max_limit_price_deviation: old.max_limit_price_deviation,
            max_market_slippage: old.max_market_slippage,
            max_abs_oi: old.max_abs_oi,
            max_abs_funding_rate: old.max_abs_funding_rate,
            initial_margin_ratio: old.initial_margin_ratio,
            maintenance_margin_ratio: old.maintenance_margin_ratio,
            impact_size: old.impact_size,
            vault_liquidity_weight: old.vault_liquidity_weight,
            vault_half_spread: old.vault_half_spread,
            vault_max_quote_size: old.vault_max_quote_size,
            vault_size_skew_factor: old.vault_size_skew_factor,
            vault_spread_skew_factor: old.vault_spread_skew_factor,
            vault_max_skew_size: old.vault_max_skew_size,
            funding_rate_multiplier: old.funding_rate_multiplier,
            bucket_sizes: old.bucket_sizes,
        };

        PAIR_PARAMS.save(storage, &pair_id, &new)?;

        update_lot_size(
            storage,
            &pair_id,
            Quantity::ZERO,
            new_lot_size,
            &new.bucket_sizes,
            None,
        )?;
    }

    Ok(())
}

fn lot_size_for(pair_id: &PairId) -> Quantity {
    match pair_id.to_string().as_str() {
        "perp/btcusd" => Quantity::new(Dec128_6::from_str("0.00001").unwrap()),
        "perp/ethusd" => Quantity::new(Dec128_6::from_str("0.0001").unwrap()),
        "perp/hypeusd" => Quantity::new(Dec128_6::from_str("0.001").unwrap()),
        "perp/solusd" => Quantity::new(Dec128_6::from_str("0.001").unwrap()),
        _ => Quantity::ZERO,
    }
}

/// Pre-upgrade types preserved here so the new code can decode chain
/// state written by an older version of the contract. Everything in this
/// module is read-only scaffolding for the migration and should not be
/// referenced from the post-upgrade code path.
mod legacy {
    use {
        dango_order_book::{Dimensionless, FundingRate, PairId, Quantity, UsdPrice, UsdValue},
        grug::Map,
        std::collections::BTreeSet,
    };

    /// Pre-upgrade Borsh layout of `PairParam`. The new shape inserts
    /// `lot_size: Quantity` between `min_order_value` and
    /// `max_limit_price_deviation`; every other field is unchanged
    /// (`min_order_value` is the post-rename name of the field formerly
    /// known as `min_order_size` — the bytes are identical). We use this
    /// shape to decode existing chain rows and then re-save through the
    /// typed `PAIR_PARAMS` map with the new layout.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct PairParam {
        pub tick_size: UsdPrice,
        pub min_order_value: UsdValue,
        pub max_limit_price_deviation: Dimensionless,
        pub max_market_slippage: Dimensionless,
        pub max_abs_oi: Quantity,
        pub max_abs_funding_rate: FundingRate,
        pub initial_margin_ratio: Dimensionless,
        pub maintenance_margin_ratio: Dimensionless,
        pub impact_size: UsdValue,
        pub vault_liquidity_weight: Dimensionless,
        pub vault_half_spread: Dimensionless,
        pub vault_max_quote_size: Quantity,
        pub vault_size_skew_factor: Dimensionless,
        pub vault_spread_skew_factor: Dimensionless,
        pub vault_max_skew_size: Quantity,
        pub funding_rate_multiplier: Dimensionless,
        pub bucket_sizes: BTreeSet<UsdPrice>,
    }

    /// Old-shape view of the same storage namespace as the typed
    /// [`dango_perps::state::PAIR_PARAMS`]. Used read-only by
    /// `upgrade_lot_size` to decode legacy rows.
    pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{Dimensionless, FundingPerUnit, FundingRate, UsdPrice, UsdValue},
        dango_perps::state::{PAIR_STATES, USER_STATES},
        dango_types::perps::{PairState, Position, UserState},
        grug::{MockStorage, btree_set},
        std::collections::BTreeMap,
    };

    fn btc() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn eth() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn unknown() -> PairId {
        "perp/xyzusd".parse().unwrap()
    }

    fn q(s: &str) -> Quantity {
        Quantity::new(Dec128_6::from_str(s).unwrap())
    }

    /// A representative pre-upgrade `PairParam` with non-default values
    /// in every field so the test can detect if any is silently dropped
    /// or shifted by the Borsh shim.
    fn sample_old() -> legacy::PairParam {
        legacy::PairParam {
            tick_size: UsdPrice::new_int(1),
            min_order_value: UsdValue::new_int(5),
            max_limit_price_deviation: Dimensionless::new_permille(100),
            max_market_slippage: Dimensionless::new_permille(100),
            max_abs_oi: Quantity::new_int(1_000_000),
            max_abs_funding_rate: FundingRate::new_permille(5),
            initial_margin_ratio: Dimensionless::new_permille(50),
            maintenance_margin_ratio: Dimensionless::new_permille(25),
            impact_size: UsdValue::new_int(50_000),
            vault_liquidity_weight: Dimensionless::new_int(1),
            vault_half_spread: Dimensionless::new_permille(10),
            vault_max_quote_size: Quantity::new_int(100),
            vault_size_skew_factor: Dimensionless::new_permille(500),
            vault_spread_skew_factor: Dimensionless::new_permille(300),
            vault_max_skew_size: Quantity::new_int(50),
            funding_rate_multiplier: Dimensionless::ONE,
            bucket_sizes: btree_set! { UsdPrice::new_int(1), UsdPrice::new_int(10) },
        }
    }

    fn save_legacy_pair(storage: &mut dyn Storage, pair_id: &PairId) {
        legacy::PAIR_PARAMS
            .save(storage, pair_id, &sample_old())
            .unwrap();
        PAIR_STATES
            .save(storage, pair_id, &PairState::default())
            .unwrap();
    }

    fn save_position(storage: &mut dyn Storage, pair_id: &PairId, byte: u8, size: &str) {
        let mut positions = BTreeMap::new();
        positions.insert(pair_id.clone(), Position {
            size: q(size),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        USER_STATES
            .save(storage, Addr::mock(byte), &UserState {
                positions,
                ..Default::default()
            })
            .unwrap();
    }

    /// The Borsh shim lifts every field from the old layout onto the new
    /// `PairParam`, inserting `lot_size` from `lot_size_for`. Nothing
    /// else changes.
    #[test]
    fn upgrade_preserves_all_pair_param_fields() {
        let mut storage = MockStorage::new();
        save_legacy_pair(&mut storage, &btc());

        upgrade_lot_size(&mut storage).unwrap();

        let new = PAIR_PARAMS.load(&storage, &btc()).unwrap();
        let old = sample_old();
        assert_eq!(new.lot_size, q("0.00001"));
        assert_eq!(new.tick_size, old.tick_size);
        assert_eq!(new.min_order_value, old.min_order_value);
        assert_eq!(new.max_limit_price_deviation, old.max_limit_price_deviation);
        assert_eq!(new.max_market_slippage, old.max_market_slippage);
        assert_eq!(new.max_abs_oi, old.max_abs_oi);
        assert_eq!(new.max_abs_funding_rate, old.max_abs_funding_rate);
        assert_eq!(new.initial_margin_ratio, old.initial_margin_ratio);
        assert_eq!(new.maintenance_margin_ratio, old.maintenance_margin_ratio);
        assert_eq!(new.impact_size, old.impact_size);
        assert_eq!(new.vault_liquidity_weight, old.vault_liquidity_weight);
        assert_eq!(new.vault_half_spread, old.vault_half_spread);
        assert_eq!(new.vault_max_quote_size, old.vault_max_quote_size);
        assert_eq!(new.vault_size_skew_factor, old.vault_size_skew_factor);
        assert_eq!(new.vault_spread_skew_factor, old.vault_spread_skew_factor);
        assert_eq!(new.vault_max_skew_size, old.vault_max_skew_size);
        assert_eq!(new.funding_rate_multiplier, old.funding_rate_multiplier);
        assert_eq!(new.bucket_sizes, old.bucket_sizes);
    }

    /// `lot_size_for` returns the right value for each chain-supported
    /// pair; unknown pairs default to zero (constraint disabled), which
    /// keeps existing positions untouched.
    #[test]
    fn lot_size_for_known_and_unknown_pairs() {
        assert_eq!(lot_size_for(&btc()), q("0.00001"));
        assert_eq!(lot_size_for(&eth()), q("0.0001"));
        assert_eq!(lot_size_for(&unknown()), Quantity::ZERO);
    }

    /// End-to-end: positions misaligned to the new BTC lot grid are
    /// truncated, and OI ends balanced. Two longs of 0.000015 against
    /// one short of 0.00003 → after truncation each long ends at
    /// 0.00001 and the short stays at 0.00003. long_oi 0.00002 vs
    /// short_oi 0.00003 → imbalance 0.00001 = one lot, trimmed off the
    /// heavier short.
    #[test]
    fn upgrade_aligns_positions_and_balances_oi() {
        let mut storage = MockStorage::new();
        save_legacy_pair(&mut storage, &btc());
        save_position(&mut storage, &btc(), 1, "0.000015");
        save_position(&mut storage, &btc(), 2, "0.000015");
        save_position(&mut storage, &btc(), 3, "-0.00003");
        PAIR_STATES
            .save(&mut storage, &btc(), &PairState {
                long_oi: q("0.00003"),
                short_oi: q("0.00003"),
                ..Default::default()
            })
            .unwrap();

        upgrade_lot_size(&mut storage).unwrap();

        let long_a = USER_STATES.load(&storage, Addr::mock(1)).unwrap();
        let long_b = USER_STATES.load(&storage, Addr::mock(2)).unwrap();
        assert_eq!(long_a.positions[&btc()].size, q("0.00001"));
        assert_eq!(long_b.positions[&btc()].size, q("0.00001"));

        let pair_state = PAIR_STATES.load(&storage, &btc()).unwrap();
        assert_eq!(pair_state.long_oi, pair_state.short_oi);
        assert_eq!(pair_state.long_oi, q("0.00002"));
    }

    /// A pair with no hardcoded `lot_size` (returns ZERO from
    /// `lot_size_for`) is migrated structurally — PairParam shape is
    /// lifted — but `update_lot_size` short-circuits (new == 0), so
    /// existing positions are untouched.
    #[test]
    fn upgrade_unknown_pair_lifts_shape_but_leaves_positions() {
        let mut storage = MockStorage::new();
        save_legacy_pair(&mut storage, &unknown());
        save_position(&mut storage, &unknown(), 1, "0.000015");

        upgrade_lot_size(&mut storage).unwrap();

        let new = PAIR_PARAMS.load(&storage, &unknown()).unwrap();
        assert_eq!(new.lot_size, Quantity::ZERO);
        // Position size kept verbatim because the constraint is disabled.
        let user = USER_STATES.load(&storage, Addr::mock(1)).unwrap();
        assert_eq!(user.positions[&unknown()].size, q("0.000015"));
    }

    /// Multiple pairs in one upgrade: each gets its own `lot_size` and
    /// its own alignment pass.
    #[test]
    fn upgrade_handles_multiple_pairs() {
        let mut storage = MockStorage::new();
        save_legacy_pair(&mut storage, &btc());
        save_legacy_pair(&mut storage, &eth());

        upgrade_lot_size(&mut storage).unwrap();

        assert_eq!(
            PAIR_PARAMS.load(&storage, &btc()).unwrap().lot_size,
            q("0.00001")
        );
        assert_eq!(
            PAIR_PARAMS.load(&storage, &eth()).unwrap().lot_size,
            q("0.0001")
        );
    }
}
