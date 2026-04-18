//! Chain upgrade handler.
//!
//! Contains both the perps price-banding / client-order-id migration and the
//! gateway rate-limit migration.

use {
    dango_gateway::{EPOCH, RATE_LIMITS, SUPPLIES},
    dango_perps::state::OrderKey,
    dango_types::{
        Dimensionless, FundingRate, Quantity, UsdPrice, UsdValue,
        config::AppConfig,
        perps::{self, ChildOrder, OrderId, PairId},
    },
    grug::{
        Addr, BlockInfo, Denom, IndexedMap, JsonDeExt, Map, MultiIndex, Order as IterationOrder,
        StdResult, Storage, Timestamp, Uint128, UniqueIndex, addr,
    },
    grug_app::{APP_CONFIG, AppResult, CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
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

/// Old gateway storage key from the previous rate-limit implementation.
const OLD_OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");

/// Bank contract's supply map (same key as `dango_bank::SUPPLIES`).
const BANK_SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

/// Legacy types matching the pre-upgrade Borsh layout.
///
/// `PairParam` before this upgrade does not contain the
/// `max_limit_price_deviation` or `max_market_slippage` fields — both
/// are introduced by the price-banding / slippage-policy PR.
///
/// `LimitOrder` before this upgrade does not contain the
/// `client_order_id` field, and `OrderIndexes` only had the `order_id`
/// and `user` indexes (the `client_order_id` `UniqueIndex` is new).
/// Both `BIDS` and `ASKS` mirror the pre-upgrade `IndexedMap` layout
/// verbatim so `legacy::BIDS.clear_all` correctly wipes the legacy
/// primary entries *and* the legacy index namespaces (`bid__id`,
/// `bid__user`, etc.) before we re-save under the new shape.
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

    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct LimitOrder {
        pub user: Addr,
        pub size: Quantity,
        pub reduce_only: bool,
        pub reserved_margin: UsdValue,
        pub created_at: Timestamp,
        pub tp: Option<ChildOrder>,
        pub sl: Option<ChildOrder>,
    }

    #[grug::index_list(OrderKey, LimitOrder)]
    pub struct OrderIndexes<'a> {
        pub order_id: UniqueIndex<'a, OrderKey, OrderId, LimitOrder>,
        pub user: MultiIndex<'a, OrderKey, Addr, LimitOrder>,
    }

    impl OrderIndexes<'static> {
        pub const fn new(
            pk_namespace: &'static str,
            order_id_namespace: &'static str,
            user_namespace: &'static str,
        ) -> Self {
            OrderIndexes {
                order_id: UniqueIndex::new(
                    |(_, _, order_id), _| *order_id,
                    pk_namespace,
                    order_id_namespace,
                ),
                user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
            }
        }
    }

    pub const BIDS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> =
        IndexedMap::new("bid", OrderIndexes::new("bid", "bid__id", "bid__user"));

    pub const ASKS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> =
        IndexedMap::new("ask", OrderIndexes::new("ask", "ask__id", "ask__user"));
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&*storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    // Perps migrations.
    {
        let mut perps_storage =
            StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &perps_address]);

        do_price_banding_upgrade(&mut perps_storage)?;
        do_client_order_id_upgrade(&mut perps_storage)?;
    }

    // Gateway migration.
    do_gateway_migration(storage)?;

    Ok(())
}

// ----------------------------- gateway migration -----------------------------

/// Migrates the gateway from the old single-epoch `outbound_quota` accumulator
/// to the new epoch-based per-user rate limiting with a 24-slot sliding window.
///
/// 1. Initialize `EPOCH` to 0.
/// 2. Snapshot `SUPPLIES` for each rate-limited denom (read from bank storage).
/// 3. Delete the defunct `outbound_quota` map.
fn do_gateway_migration(storage: Box<dyn Storage>) -> AppResult<()> {
    let config = CONFIG.load(&*storage)?;
    let app_config: AppConfig = APP_CONFIG.load(&*storage)?.deserialize_json()?;

    let bank_addr = config.bank;
    let gateway_addr = app_config.addresses.gateway;

    let bank_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &bank_addr]);
    let mut gw_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &gateway_addr]);

    // 1. Initialize the epoch counter.
    EPOCH.save(&mut gw_storage, &0)?;

    tracing::info!("Initialized EPOCH to 0");

    // 2. Snapshot supply for each rate-limited denom by reading from the bank.
    let rate_limits = RATE_LIMITS.load(&gw_storage)?;

    for denom in rate_limits.keys() {
        let supply = BANK_SUPPLIES
            .may_load(&bank_storage, denom)?
            .unwrap_or_default();

        SUPPLIES.save(&mut gw_storage, denom, &supply)?;

        tracing::info!(denom = %denom, supply = %supply, "Snapshotted supply");
    }

    // 3. Delete the old `outbound_quota` map.
    OLD_OUTBOUND_QUOTAS.clear(&mut gw_storage, None, None);

    tracing::info!("Cleared old outbound_quota map");

    Ok(())
}

// ----------------------------- perps migrations ------------------------------

fn do_price_banding_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
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

/// Rewrite every resting `LimitOrder` in `BIDS` / `ASKS` from the
/// pre-`client_order_id` Borsh layout to the new layout, with
/// `client_order_id: None` for all migrated orders. Existing orders
/// were submitted before the feature shipped, so none of them carry a
/// client id; the new `client_order_id` index stays empty for them.
///
/// The pattern is **read-via-legacy → clear-via-legacy → save-via-new**:
///
/// 1. `legacy_map.range` deserializes the on-disk bytes through the
///    old Borsh schema.
/// 2. `legacy_map.clear_all` wipes the primary namespace *and* the
///    legacy index namespaces (`bid__id`, `bid__user`, …). Without
///    this, `IndexedMap::save` below would hit
///    `StdError::duplicate_data` on the `order_id` `UniqueIndex` while
///    rebuilding it.
/// 3. `new_map.save` re-populates the primary entry and rebuilds all
///    three new indexes (`order_id`, `user`, `client_order_id`).
fn do_client_order_id_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    let bid_count = migrate_book(storage, &legacy::BIDS, &dango_perps::state::BIDS)?;
    let ask_count = migrate_book(storage, &legacy::ASKS, &dango_perps::state::ASKS)?;

    tracing::info!(
        "Migrated {bid_count} resting bids and {ask_count} resting asks (added client_order_id = None)"
    );

    Ok(())
}

fn migrate_book(
    storage: &mut dyn Storage,
    legacy_map: &IndexedMap<OrderKey, legacy::LimitOrder, legacy::OrderIndexes>,
    new_map: &IndexedMap<
        OrderKey,
        dango_types::perps::LimitOrder,
        dango_perps::state::OrderIndexes,
    >,
) -> StdResult<usize> {
    let entries: Vec<_> = legacy_map
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let count = entries.len();

    legacy_map.clear_all(storage);

    for (key, old) in entries {
        let new = dango_types::perps::LimitOrder {
            user: old.user,
            size: old.size,
            reduce_only: old.reduce_only,
            reserved_margin: old.reserved_margin,
            created_at: old.created_at,
            tp: old.tp,
            sl: old.sl,
            client_order_id: None,
        };
        new_map.save(storage, key, &new)?;
    }

    Ok(count)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{MockStorage, Uint64, addr, btree_set},
    };

    const USER_A: Addr = addr!("000000000000000000000000000000000000aaaa");
    const USER_B: Addr = addr!("000000000000000000000000000000000000bbbb");

    fn eth_pair() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn btc_pair() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn legacy_limit_order(user: Addr, size: i128, reserved_margin: i128) -> legacy::LimitOrder {
        legacy::LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
            created_at: Timestamp::from_nanos(123_000_000),
            tp: None,
            sl: None,
        }
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

        do_price_banding_upgrade(&mut storage).unwrap();

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
        do_price_banding_upgrade(&mut storage).unwrap();
    }

    /// `do_client_order_id_upgrade` rewrites every legacy resting order
    /// into the new shape with `client_order_id == None`, preserves the
    /// other fields verbatim, and leaves the `order_id` index pointing
    /// at the new entry.
    #[test]
    fn client_order_id_migration_rewrites_legacy_orders() {
        let mut storage = MockStorage::new();

        // Two legacy bids and one legacy ask, spanning two users.
        let bid_a = legacy_limit_order(USER_A, 10, 100);
        let bid_b = legacy_limit_order(USER_B, 5, 50);
        let ask_a = legacy_limit_order(USER_A, -7, 70);

        legacy::BIDS
            .save(
                &mut storage,
                (eth_pair(), UsdPrice::new_int(2_000), Uint64::new(1)),
                &bid_a,
            )
            .unwrap();
        legacy::BIDS
            .save(
                &mut storage,
                (eth_pair(), UsdPrice::new_int(1_950), Uint64::new(2)),
                &bid_b,
            )
            .unwrap();
        legacy::ASKS
            .save(
                &mut storage,
                (eth_pair(), UsdPrice::new_int(2_100), Uint64::new(3)),
                &ask_a,
            )
            .unwrap();

        do_client_order_id_upgrade(&mut storage).unwrap();

        // All three orders are now reachable via the new `IndexedMap`.
        for (book, order_id, expected_size, expected_user, expected_margin) in [
            (
                &dango_perps::state::BIDS,
                Uint64::new(1),
                Quantity::new_int(10),
                USER_A,
                UsdValue::new_int(100),
            ),
            (
                &dango_perps::state::BIDS,
                Uint64::new(2),
                Quantity::new_int(5),
                USER_B,
                UsdValue::new_int(50),
            ),
        ] {
            let (_key, order) = book
                .idx
                .order_id
                .may_load(&storage, order_id)
                .unwrap()
                .expect("post-migration order missing from order_id index");
            assert_eq!(order.user, expected_user);
            assert_eq!(order.size, expected_size);
            assert_eq!(order.reserved_margin, expected_margin);
            assert_eq!(order.client_order_id, None);
            assert_eq!(order.created_at, Timestamp::from_nanos(123_000_000));
        }

        let (_key, ask_order) = dango_perps::state::ASKS
            .idx
            .order_id
            .may_load(&storage, Uint64::new(3))
            .unwrap()
            .unwrap();
        assert_eq!(ask_order.user, USER_A);
        assert_eq!(ask_order.size, Quantity::new_int(-7));
        assert_eq!(ask_order.client_order_id, None);

        // The new `client_order_id` index is empty for migrated orders.
        let any_cid = dango_perps::state::BIDS
            .idx
            .client_order_id
            .keys(&storage, None, None, grug::Order::Ascending)
            .next();
        assert!(
            any_cid.is_none(),
            "new client_order_id index should be empty"
        );
    }

    #[test]
    fn client_order_id_migration_no_orders_is_noop() {
        let mut storage = MockStorage::new();
        do_client_order_id_upgrade(&mut storage).unwrap();
    }

    /// `do_upgrade` chains both migrations: legacy `PairParam`s are
    /// rewritten *and* legacy resting orders are rewritten, all under
    /// one upgrade boundary.
    #[test]
    fn do_upgrade_runs_both_migrations() {
        let mut storage = MockStorage::new();

        // Seed legacy state for both migrations.
        legacy::PAIR_PARAMS
            .save(&mut storage, &eth_pair(), &legacy_pair_param(1))
            .unwrap();
        legacy::BIDS
            .save(
                &mut storage,
                (eth_pair(), UsdPrice::new_int(2_000), Uint64::new(1)),
                &legacy_limit_order(USER_A, 10, 100),
            )
            .unwrap();

        // Run both migrations sequentially (mirrors `do_upgrade`).
        do_price_banding_upgrade(&mut storage).unwrap();
        do_client_order_id_upgrade(&mut storage).unwrap();

        // PairParam migrated.
        let pair = dango_perps::state::PAIR_PARAMS
            .load(&storage, &eth_pair())
            .unwrap();
        assert_eq!(
            pair.max_limit_price_deviation,
            MIGRATION_MAX_LIMIT_PRICE_DEVIATION
        );
        assert_eq!(pair.max_market_slippage, MIGRATION_MAX_MARKET_SLIPPAGE);

        // Order migrated.
        let (_key, order) = dango_perps::state::BIDS
            .idx
            .order_id
            .may_load(&storage, Uint64::new(1))
            .unwrap()
            .unwrap();
        assert_eq!(order.user, USER_A);
        assert_eq!(order.client_order_id, None);
    }
}
