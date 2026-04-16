use {
    dango_types::{
        Dimensionless, FundingRate, Quantity, UsdPrice, UsdValue,
        perps::{self, PairId},
    },
    grug::{Addr, BlockInfo, Order as IterationOrder, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeSet,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Default `max_limit_price_deviation` applied to every pair during the
/// migration. 5% matches Binance USD-M futures' `PERCENT_PRICE` filter
/// for BTCUSDT / ETHUSDT / SOLUSDT (multiplierUp 1.05, multiplierDown
/// 0.95), and sits well above the vault's `vault_half_spread × (1 +
/// vault_spread_skew_factor)` upper bound for any reasonable
/// parameterization. Governance is expected to tighten this per-pair
/// via `Configure` after the upgrade completes.
const MIGRATION_MAX_LIMIT_PRICE_DEVIATION: Dimensionless = Dimensionless::new_percent(5);

/// Default `max_market_slippage` applied to every pair during the
/// migration. 5% matches Binance USD-M futures' `marketTakeBound` for
/// BTCUSDT / ETHUSDT / SOLUSDT, the largest centralized perp venue by
/// volume. Tighter than dYdX v4 (10%) and Hyperliquid TP/SL (10%) but
/// the symmetry with `max_limit_price_deviation` is intentional.
/// Governance tightens per-pair via `Configure` after the upgrade.
const MIGRATION_MAX_MARKET_SLIPPAGE: Dimensionless = Dimensionless::new_percent(5);

/// Legacy types matching the pre-upgrade Borsh layout.
///
/// `PairParam` before this upgrade does not contain the
/// `max_limit_price_deviation` or `max_market_slippage` fields —
/// both are introduced by the price-banding / slippage-policy PR.
mod legacy {
    use super::*;

    pub const PAIR_PARAMS: grug::Map<&PairId, PairParam> = grug::Map::new("pair_param");

    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct PairParam {
        pub tick_size: UsdPrice,
        pub min_order_size: UsdValue,
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
        pub bucket_sizes: BTreeSet<UsdPrice>,
    }
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    Ok(_do_upgrade(&mut storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    let old_params: Vec<_> = legacy::PAIR_PARAMS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let count = old_params.len();

    for (pair_id, old) in old_params {
        let new = perps::PairParam {
            tick_size: old.tick_size,
            min_order_size: old.min_order_size,
            // New fields — permissive defaults; governance tightens per-pair.
            max_limit_price_deviation: MIGRATION_MAX_LIMIT_PRICE_DEVIATION,
            max_market_slippage: MIGRATION_MAX_MARKET_SLIPPAGE,
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
            bucket_sizes: old.bucket_sizes,
        };

        dango_perps::state::PAIR_PARAMS.save(storage, &pair_id, &new)?;
    }

    tracing::info!(
        "Migrated {count} PairParam entries (added max_limit_price_deviation = {MIGRATION_MAX_LIMIT_PRICE_DEVIATION}, max_market_slippage = {MIGRATION_MAX_MARKET_SLIPPAGE})"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{MockStorage, btree_set},
    };

    fn eth_pair() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn btc_pair() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn legacy_pair_param(tick_size_int: i128) -> legacy::PairParam {
        legacy::PairParam {
            tick_size: UsdPrice::new_int(tick_size_int),
            min_order_size: UsdValue::new_int(100),
            max_abs_oi: Quantity::new_int(1_000_000),
            max_abs_funding_rate: FundingRate::new_permille(500),
            initial_margin_ratio: Dimensionless::new_permille(100),
            maintenance_margin_ratio: Dimensionless::new_permille(50),
            impact_size: UsdValue::new_int(10_000),
            vault_liquidity_weight: Dimensionless::new_int(1),
            vault_half_spread: Dimensionless::new_permille(10),
            vault_max_quote_size: Quantity::new_int(100),
            vault_size_skew_factor: Dimensionless::new_permille(200),
            vault_spread_skew_factor: Dimensionless::new_permille(300),
            vault_max_skew_size: Quantity::new_int(500),
            bucket_sizes: btree_set! { UsdPrice::new_int(1), UsdPrice::new_int(10) },
        }
    }

    #[test]
    fn migration_copies_fields_and_sets_new_default() {
        let mut storage = MockStorage::new();

        let eth = legacy_pair_param(1);
        let btc = legacy_pair_param(2);

        legacy::PAIR_PARAMS
            .save(&mut storage, &eth_pair(), &eth)
            .unwrap();
        legacy::PAIR_PARAMS
            .save(&mut storage, &btc_pair(), &btc)
            .unwrap();

        _do_upgrade(&mut storage).unwrap();

        let migrated_eth = dango_perps::state::PAIR_PARAMS
            .load(&storage, &eth_pair())
            .unwrap();
        let migrated_btc = dango_perps::state::PAIR_PARAMS
            .load(&storage, &btc_pair())
            .unwrap();

        // All pre-existing fields are preserved.
        assert_eq!(migrated_eth.tick_size, eth.tick_size);
        assert_eq!(migrated_eth.min_order_size, eth.min_order_size);
        assert_eq!(migrated_eth.max_abs_oi, eth.max_abs_oi);
        assert_eq!(migrated_eth.max_abs_funding_rate, eth.max_abs_funding_rate);
        assert_eq!(migrated_eth.initial_margin_ratio, eth.initial_margin_ratio);
        assert_eq!(
            migrated_eth.maintenance_margin_ratio,
            eth.maintenance_margin_ratio
        );
        assert_eq!(migrated_eth.impact_size, eth.impact_size);
        assert_eq!(
            migrated_eth.vault_liquidity_weight,
            eth.vault_liquidity_weight
        );
        assert_eq!(migrated_eth.vault_half_spread, eth.vault_half_spread);
        assert_eq!(migrated_eth.vault_max_quote_size, eth.vault_max_quote_size);
        assert_eq!(
            migrated_eth.vault_size_skew_factor,
            eth.vault_size_skew_factor
        );
        assert_eq!(
            migrated_eth.vault_spread_skew_factor,
            eth.vault_spread_skew_factor
        );
        assert_eq!(migrated_eth.vault_max_skew_size, eth.vault_max_skew_size);
        assert_eq!(migrated_eth.bucket_sizes, eth.bucket_sizes);

        // New fields populated with the migration defaults for both pairs.
        assert_eq!(
            migrated_eth.max_limit_price_deviation,
            MIGRATION_MAX_LIMIT_PRICE_DEVIATION
        );
        assert_eq!(
            migrated_btc.max_limit_price_deviation,
            MIGRATION_MAX_LIMIT_PRICE_DEVIATION
        );
        assert_eq!(
            migrated_eth.max_market_slippage,
            MIGRATION_MAX_MARKET_SLIPPAGE
        );
        assert_eq!(
            migrated_btc.max_market_slippage,
            MIGRATION_MAX_MARKET_SLIPPAGE
        );
        assert_eq!(migrated_btc.tick_size, btc.tick_size);
    }

    #[test]
    fn migration_with_no_pairs_is_noop() {
        let mut storage = MockStorage::new();
        _do_upgrade(&mut storage).unwrap();
    }
}
