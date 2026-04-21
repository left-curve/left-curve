use {
    dango_perps::state::OrderKey,
    dango_types::{
        Dimensionless, FundingRate, Quantity, UsdPrice, UsdValue,
        account_factory::UserIndex,
        perps::{self, ChildOrder, FillId, OrderId, PairId, RateSchedule},
    },
    grug::{
        Addr, BlockInfo, Duration, IndexedMap, MultiIndex, NumberConst, Order as IterationOrder,
        StdResult, Storage, Timestamp, UniqueIndex, addr,
    },
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::{BTreeMap, BTreeSet},
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

/// Initial `max_action_batch_size` applied to the global `Param` during
/// the migration. 5 matches Binance USD-M futures' `batchOrders` cap —
/// conservative starting point; governance is expected to raise it via
/// `Configure` once traffic patterns are understood.
const MIGRATION_MAX_ACTION_BATCH_SIZE: usize = 5;

/// Default `funding_rate_multiplier` applied to every pair during the
/// migration. `1` reproduces the pre-multiplier funding behavior —
/// funding continues to be computed as `-halfSpread × skew ×
/// spreadSkewFactor` at the upgrade boundary, so there is no
/// discontinuity for positions active across the upgrade. Governance
/// tunes per-pair via `Configure` after the upgrade completes.
const MIGRATION_FUNDING_RATE_MULTIPLIER: Dimensionless = Dimensionless::ONE;

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

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    pub const PAIR_PARAMS: grug::Map<&PairId, PairParam> = grug::Map::new("pair_param");

    /// Pre-upgrade layout of the global `Param`. Lacks the
    /// `max_action_batch_size` field introduced by the batch-action PR;
    /// order of the remaining fields must match the on-disk Borsh layout
    /// exactly.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct Param {
        pub max_unlocks: usize,
        pub max_open_orders: usize,
        pub maker_fee_rates: RateSchedule,
        pub taker_fee_rates: RateSchedule,
        pub protocol_fee_rate: Dimensionless,
        pub liquidation_fee_rate: Dimensionless,
        pub liquidation_buffer_ratio: Dimensionless,
        pub funding_period: Duration,
        pub vault_total_weight: Dimensionless,
        pub vault_cooldown_period: Duration,
        pub referral_active: bool,
        pub min_referrer_volume: UsdValue,
        pub referrer_commission_rates: RateSchedule,
        pub vault_deposit_cap: Option<UsdValue>,
    }

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

    /// Pre-upgrade layout of `UserReferralData`. Lacks the
    /// `cumulative_global_active_referees` field and uses the old name
    /// `cumulative_active_referees` (same Borsh position).
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct UserReferralData {
        pub volume: UsdValue,
        pub commission_shared_by_referrer: UsdValue,
        pub referee_count: u32,
        pub referees_volume: UsdValue,
        pub commission_earned_from_referees: UsdValue,
        pub cumulative_active_referees: u32,
    }

    pub const USER_REFERRAL_DATA: grug::Map<(UserIndex, Timestamp), UserReferralData> =
        grug::Map::new("ref_data");
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    do_pair_param_and_param_upgrades(&mut storage)?;
    do_client_order_id_upgrade(&mut storage)?;
    do_fill_id_upgrade(&mut storage)?;
    do_reserved_margin_rounding_error_fix(&mut storage)?;
    do_referral_activated_referees_upgrade(&mut storage)?;

    Ok(())
}

/// Borsh-layout-breaking migrations to `PairParam` and the global
/// `Param`, bundled into one upgrade event:
///
/// 1. Price banding — adds `max_limit_price_deviation` and
///    `max_market_slippage` to every `PairParam`.
/// 2. Funding rate multiplier — adds `funding_rate_multiplier` to
///    every `PairParam`, seeded at `1` so funding behavior is
///    unchanged at the upgrade boundary.
/// 3. Batch action size cap — adds `max_action_batch_size` to the
///    global `Param`.
///
/// All three are pure additions to their respective struct layouts, so
/// the migration is a straightforward read-legacy / write-new
/// roundtrip.
fn do_pair_param_and_param_upgrades(storage: &mut dyn Storage) -> StdResult<()> {
    // 1. PairParam: add price-banding fields.
    let old_pair_params: Vec<_> = legacy::PAIR_PARAMS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let pair_count = old_pair_params.len();

    for (pair_id, old) in old_pair_params {
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
            // Identity multiplier preserves pre-upgrade funding behavior.
            funding_rate_multiplier: MIGRATION_FUNDING_RATE_MULTIPLIER,
            bucket_sizes: old.bucket_sizes,
        };

        dango_perps::state::PAIR_PARAMS.save(storage, &pair_id, &new)?;
    }

    tracing::info!(
        "Migrated {pair_count} PairParam entries (added max_limit_price_deviation = {MIGRATION_MAX_LIMIT_PRICE_DEVIATION}, max_market_slippage = {MIGRATION_MAX_MARKET_SLIPPAGE}, funding_rate_multiplier = {MIGRATION_FUNDING_RATE_MULTIPLIER})"
    );

    // 2. Param: add `max_action_batch_size`.
    let old_param = legacy::PARAM.load(storage)?;
    let new_param = perps::Param {
        max_unlocks: old_param.max_unlocks,
        max_open_orders: old_param.max_open_orders,
        maker_fee_rates: old_param.maker_fee_rates,
        taker_fee_rates: old_param.taker_fee_rates,
        protocol_fee_rate: old_param.protocol_fee_rate,
        liquidation_fee_rate: old_param.liquidation_fee_rate,
        liquidation_buffer_ratio: old_param.liquidation_buffer_ratio,
        funding_period: old_param.funding_period,
        vault_total_weight: old_param.vault_total_weight,
        vault_cooldown_period: old_param.vault_cooldown_period,
        referral_active: old_param.referral_active,
        min_referrer_volume: old_param.min_referrer_volume,
        referrer_commission_rates: old_param.referrer_commission_rates,
        vault_deposit_cap: old_param.vault_deposit_cap,
        // New field.
        max_action_batch_size: MIGRATION_MAX_ACTION_BATCH_SIZE,
    };
    dango_perps::state::PARAM.save(storage, &new_param)?;

    tracing::info!(
        "Migrated global Param (added max_action_batch_size = {MIGRATION_MAX_ACTION_BATCH_SIZE})"
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

/// Seed `NEXT_FILL_ID` with `FillId::ONE` so that the counter exists for
/// post-upgrade `load` calls. Fills executed before the upgrade have no
/// `fill_id` in their emitted `OrderFilled` events and are not backfilled —
/// downstream consumers treat a missing field as "pre-v0.15.0".
///
/// Idempotency: this handler seeds a fresh counter. If it were ever rerun
/// on a chain that already has `NEXT_FILL_ID` populated, blindly saving
/// `FillId::ONE` would reset a live counter and cause duplicate fill ids
/// to be emitted. The assertion below makes the invariant explicit —
/// `assert!` is deliberate: the upgrade framework has no recovery path,
/// and silently skipping would hide a serious control-flow bug.
fn do_fill_id_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    assert!(
        dango_perps::state::NEXT_FILL_ID
            .may_load(storage)?
            .is_none(),
        "NEXT_FILL_ID already initialized — do_fill_id_upgrade must not run twice",
    );
    dango_perps::state::NEXT_FILL_ID.save(storage, &FillId::ONE)?;
    tracing::info!("Initialized NEXT_FILL_ID to 1");
    Ok(())
}

/// Repair `UserState::reserved_margin` drift caused by a rounding bug in
/// the proportional margin-release formula in `submit_order::match_order`
/// (fixed in the same release). Pre-fix, each fully-filled maker order
/// could orphan up to a few ULPs of reserved margin on the user side.
/// A mainnet survey (Apr 2026) found hundreds of users with residuals
/// ≤ ~0.04 USD.
///
/// For every user we recompute `reserved_margin` as the sum of
/// `LimitOrder::reserved_margin` across their resting bids and asks,
/// overwriting the field if it disagrees. This restores the invariant
/// `user_state.reserved_margin == sum(open orders' reserved_margin)` at
/// the upgrade boundary; from there the fixed matcher keeps it intact.
///
/// Must run *after* `do_client_order_id_upgrade` so that the books are
/// already in the new `dango_perps::state::{BIDS, ASKS}` layout — the
/// per-user `MultiIndex` used below reads through the new shape.
fn do_reserved_margin_rounding_error_fix(storage: &mut dyn Storage) -> StdResult<()> {
    // Snapshot users first — we cannot iterate `USER_STATES` and mutate
    // it in the same pass.
    let users: Vec<_> = dango_perps::state::USER_STATES
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let mut fixed = 0usize;

    for (user, mut user_state) in users {
        let mut actual = UsdValue::ZERO;

        for res in dango_perps::state::BIDS.idx.user.prefix(user).range(
            storage,
            None,
            None,
            IterationOrder::Ascending,
        ) {
            let (_, order) = res?;
            actual.checked_add_assign(order.reserved_margin)?;
        }

        for res in dango_perps::state::ASKS.idx.user.prefix(user).range(
            storage,
            None,
            None,
            IterationOrder::Ascending,
        ) {
            let (_, order) = res?;
            actual.checked_add_assign(order.reserved_margin)?;
        }

        if actual != user_state.reserved_margin {
            tracing::info!(
                %user,
                old = %user_state.reserved_margin,
                new = %actual,
                "Fixed reserved_margin rounding drift",
            );
            user_state.reserved_margin = actual;
            dango_perps::state::USER_STATES.save(storage, user, &user_state)?;
            fixed += 1;
        }
    }

    tracing::info!("Reserved_margin rounding-error fix complete: {fixed} user(s) updated");

    Ok(())
}

/// Migrate `USER_REFERRAL_DATA` from the old Borsh layout (missing
/// `cumulative_global_active_referees`) to the new layout. Computes the
/// activated-referee count for each referrer by scanning
/// `REFERRER_TO_REFEREE_STATISTICS` for referees whose `last_day_active` is
/// non-zero (indicating at least one trade).
///
/// For each referrer we set `cumulative_global_active_referees` on every
/// existing bucket to the computed total. This is slightly imprecise for
/// historical buckets (we back-project the current total), but the field
/// is monotonically non-decreasing and there is no way to reconstruct
/// the exact per-day activation history retroactively.
fn do_referral_activated_referees_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    // 1. Compute activated referees per referrer from REFERRER_TO_REFEREE_STATISTICS.
    let mut activated_per_referrer: BTreeMap<UserIndex, u32> = BTreeMap::new();

    for res in dango_perps::state::REFERRER_TO_REFEREE_STATISTICS.range(
        storage,
        None,
        None,
        IterationOrder::Ascending,
    ) {
        let ((referrer, _referee), stats) = res?;
        if stats.last_day_active != Timestamp::ZERO {
            *activated_per_referrer.entry(referrer).or_default() += 1;
        }
    }

    // 2. Read all legacy USER_REFERRAL_DATA entries.
    let entries: Vec<_> = legacy::USER_REFERRAL_DATA
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let entry_count = entries.len();

    // 3. Rewrite each entry with the new layout.
    for ((user, ts), old) in entries {
        let activated = activated_per_referrer.get(&user).copied().unwrap_or(0);

        let new = perps::UserReferralData {
            volume: old.volume,
            commission_shared_by_referrer: old.commission_shared_by_referrer,
            referee_count: old.referee_count,
            referees_volume: old.referees_volume,
            commission_earned_from_referees: old.commission_earned_from_referees,
            cumulative_daily_active_referees: old.cumulative_active_referees,
            cumulative_global_active_referees: activated,
        };

        dango_perps::state::USER_REFERRAL_DATA.save(storage, (user, ts), &new)?;
    }

    tracing::info!(
        "Migrated {entry_count} UserReferralData entries (added cumulative_global_active_referees, {} referrers have activated referees)",
        activated_per_referrer.len()
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

    fn legacy_param() -> legacy::Param {
        legacy::Param {
            max_unlocks: 10,
            max_open_orders: 100,
            maker_fee_rates: RateSchedule::default(),
            taker_fee_rates: RateSchedule::default(),
            protocol_fee_rate: Dimensionless::ZERO,
            liquidation_fee_rate: Dimensionless::new_permille(10),
            liquidation_buffer_ratio: Dimensionless::ZERO,
            funding_period: Duration::from_hours(1),
            vault_total_weight: Dimensionless::ZERO,
            vault_cooldown_period: Duration::from_days(1),
            referral_active: true,
            min_referrer_volume: UsdValue::ZERO,
            referrer_commission_rates: RateSchedule::default(),
            vault_deposit_cap: None,
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

    /// Build a current-shape `LimitOrder` with `reserved_margin` given
    /// directly in inner (Dec128_6) units — useful for exercising the
    /// rounding-error fix with fractional values.
    fn limit_order_with_reserved_raw(
        user: Addr,
        size: i128,
        reserved_margin_raw: i128,
    ) -> dango_types::perps::LimitOrder {
        dango_types::perps::LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_raw(reserved_margin_raw),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
        }
    }

    fn plant_bid(
        storage: &mut dyn Storage,
        pair: PairId,
        price: i128,
        id: u64,
        order: &dango_types::perps::LimitOrder,
    ) {
        // Buy-side orders are stored under the inverted price so iteration
        // yields the highest-priced bids first.
        let stored_price = !UsdPrice::new_int(price);
        dango_perps::state::BIDS
            .save(storage, (pair, stored_price, Uint64::new(id)), order)
            .unwrap();
    }

    fn plant_ask(
        storage: &mut dyn Storage,
        pair: PairId,
        price: i128,
        id: u64,
        order: &dango_types::perps::LimitOrder,
    ) {
        dango_perps::state::ASKS
            .save(
                storage,
                (pair, UsdPrice::new_int(price), Uint64::new(id)),
                order,
            )
            .unwrap();
    }

    fn plant_user_state_with_reserved_raw(
        storage: &mut dyn Storage,
        user: Addr,
        reserved_raw: i128,
    ) {
        let state = dango_types::perps::UserState {
            reserved_margin: UsdValue::new_raw(reserved_raw),
            ..Default::default()
        };
        dango_perps::state::USER_STATES
            .save(storage, user, &state)
            .unwrap();
    }

    #[test]
    fn migration_copies_fields_and_sets_new_default() {
        let mut storage = MockStorage::new();

        let eth = legacy_pair_param(1);
        let btc = legacy_pair_param(2);
        let old_param = legacy_param();

        legacy::PAIR_PARAMS
            .save(&mut storage, &eth_pair(), &eth)
            .unwrap();
        legacy::PAIR_PARAMS
            .save(&mut storage, &btc_pair(), &btc)
            .unwrap();
        legacy::PARAM.save(&mut storage, &old_param).unwrap();

        do_pair_param_and_param_upgrades(&mut storage).unwrap();

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
        assert_eq!(
            migrated_eth.funding_rate_multiplier,
            MIGRATION_FUNDING_RATE_MULTIPLIER
        );
        assert_eq!(
            migrated_btc.funding_rate_multiplier,
            MIGRATION_FUNDING_RATE_MULTIPLIER
        );
        assert_eq!(migrated_btc.tick_size, btc.tick_size);

        // The global `Param` preserves every old field and gains
        // `max_action_batch_size = MIGRATION_MAX_ACTION_BATCH_SIZE`.
        let migrated_param = dango_perps::state::PARAM.load(&storage).unwrap();
        assert_eq!(migrated_param.max_unlocks, old_param.max_unlocks);
        assert_eq!(migrated_param.max_open_orders, old_param.max_open_orders);
        assert_eq!(migrated_param.maker_fee_rates, old_param.maker_fee_rates);
        assert_eq!(migrated_param.taker_fee_rates, old_param.taker_fee_rates);
        assert_eq!(
            migrated_param.protocol_fee_rate,
            old_param.protocol_fee_rate
        );
        assert_eq!(
            migrated_param.liquidation_fee_rate,
            old_param.liquidation_fee_rate
        );
        assert_eq!(
            migrated_param.liquidation_buffer_ratio,
            old_param.liquidation_buffer_ratio
        );
        assert_eq!(migrated_param.funding_period, old_param.funding_period);
        assert_eq!(
            migrated_param.vault_total_weight,
            old_param.vault_total_weight
        );
        assert_eq!(
            migrated_param.vault_cooldown_period,
            old_param.vault_cooldown_period
        );
        assert_eq!(migrated_param.referral_active, old_param.referral_active);
        assert_eq!(
            migrated_param.min_referrer_volume,
            old_param.min_referrer_volume
        );
        assert_eq!(
            migrated_param.referrer_commission_rates,
            old_param.referrer_commission_rates
        );
        assert_eq!(
            migrated_param.vault_deposit_cap,
            old_param.vault_deposit_cap
        );
        assert_eq!(
            migrated_param.max_action_batch_size,
            MIGRATION_MAX_ACTION_BATCH_SIZE
        );
    }

    /// With no legacy pairs to migrate, the Param migration still runs
    /// and backfills `max_action_batch_size`.
    #[test]
    fn migration_with_no_pairs_still_migrates_param() {
        let mut storage = MockStorage::new();
        legacy::PARAM.save(&mut storage, &legacy_param()).unwrap();
        do_pair_param_and_param_upgrades(&mut storage).unwrap();
        assert_eq!(
            dango_perps::state::PARAM
                .load(&storage)
                .unwrap()
                .max_action_batch_size,
            MIGRATION_MAX_ACTION_BATCH_SIZE
        );
    }

    /// Focused regression: every migrated pair gets
    /// `funding_rate_multiplier = 1` so funding behavior is unchanged at
    /// the upgrade boundary. Decoupled from
    /// `migration_copies_fields_and_sets_new_default` so a constant
    /// drift fails with a targeted message.
    #[test]
    fn migration_sets_funding_rate_multiplier_to_one() {
        let mut storage = MockStorage::new();

        legacy::PAIR_PARAMS
            .save(&mut storage, &eth_pair(), &legacy_pair_param(1))
            .unwrap();
        legacy::PAIR_PARAMS
            .save(&mut storage, &btc_pair(), &legacy_pair_param(2))
            .unwrap();
        legacy::PARAM.save(&mut storage, &legacy_param()).unwrap();

        do_pair_param_and_param_upgrades(&mut storage).unwrap();

        let eth = dango_perps::state::PAIR_PARAMS
            .load(&storage, &eth_pair())
            .unwrap();
        let btc = dango_perps::state::PAIR_PARAMS
            .load(&storage, &btc_pair())
            .unwrap();

        assert_eq!(eth.funding_rate_multiplier, Dimensionless::ONE);
        assert_eq!(btc.funding_rate_multiplier, Dimensionless::ONE);
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

    /// `do_fill_id_upgrade` seeds `NEXT_FILL_ID` to 1 so post-upgrade
    /// `load` calls in the matching engine succeed.
    #[test]
    fn fill_id_upgrade_initializes_counter_to_one() {
        let mut storage = MockStorage::new();

        // Before the upgrade, the counter does not exist.
        assert!(
            dango_perps::state::NEXT_FILL_ID
                .may_load(&storage)
                .unwrap()
                .is_none()
        );

        do_fill_id_upgrade(&mut storage).unwrap();

        assert_eq!(
            dango_perps::state::NEXT_FILL_ID.load(&storage).unwrap(),
            FillId::ONE
        );
    }

    /// Running `do_fill_id_upgrade` a second time on a chain that already
    /// has a live `NEXT_FILL_ID` must panic rather than reset the counter.
    /// A silent overwrite would cause the matching engine to emit fill
    /// ids that collide with previously-emitted ones.
    #[test]
    #[should_panic(expected = "NEXT_FILL_ID already initialized")]
    fn fill_id_upgrade_rejects_rerun() {
        let mut storage = MockStorage::new();

        // Simulate a chain that already has the counter advanced past 1.
        dango_perps::state::NEXT_FILL_ID
            .save(&mut storage, &Uint64::new(42))
            .unwrap();

        // A rerun must panic.
        do_fill_id_upgrade(&mut storage).unwrap();
    }

    /// With a drifted `USER_A` (reserved_margin strictly greater than the
    /// sum of their order's reserved_margin) and a consistent `USER_B`,
    /// the fix rewrites `USER_A`'s field to the true sum and leaves
    /// `USER_B` untouched.
    #[test]
    fn reserved_margin_fix_repairs_drift() {
        let mut storage = MockStorage::new();

        // USER_A: one ask with reserved_margin = 21.405596 USD, but
        // user_state says 21.406601 USD — a 1005-ULP orphan.
        let ask_a = limit_order_with_reserved_raw(USER_A, -1, 21_405_596);
        plant_ask(&mut storage, eth_pair(), 2_340, 1, &ask_a);
        plant_user_state_with_reserved_raw(&mut storage, USER_A, 21_406_601);

        // USER_B: a bid + an ask summing to 10 USD, user_state matches.
        let bid_b = limit_order_with_reserved_raw(USER_B, 1, 6_000_000);
        let ask_b = limit_order_with_reserved_raw(USER_B, -1, 4_000_000);
        plant_bid(&mut storage, btc_pair(), 50_000, 2, &bid_b);
        plant_ask(&mut storage, btc_pair(), 51_000, 3, &ask_b);
        plant_user_state_with_reserved_raw(&mut storage, USER_B, 10_000_000);

        do_reserved_margin_rounding_error_fix(&mut storage).unwrap();

        let after_a = dango_perps::state::USER_STATES
            .load(&storage, USER_A)
            .unwrap();
        assert_eq!(after_a.reserved_margin, UsdValue::new_raw(21_405_596));

        let after_b = dango_perps::state::USER_STATES
            .load(&storage, USER_B)
            .unwrap();
        assert_eq!(after_b.reserved_margin, UsdValue::new_raw(10_000_000));
    }

    /// Users whose orders have all been fully filled can still carry a
    /// residual reserved_margin. With no orders left, the fix resets
    /// the field to zero.
    #[test]
    fn reserved_margin_fix_resets_to_zero_when_no_orders() {
        let mut storage = MockStorage::new();

        // User with 0.034211 USD reserved but no open orders — matches
        // the shape of several mainnet inconsistencies from the survey.
        plant_user_state_with_reserved_raw(&mut storage, USER_A, 34_211);

        do_reserved_margin_rounding_error_fix(&mut storage).unwrap();

        let after = dango_perps::state::USER_STATES
            .load(&storage, USER_A)
            .unwrap();
        assert_eq!(after.reserved_margin, UsdValue::ZERO);
    }

    /// When every user's reserved_margin already matches the sum of
    /// their orders' reserved_margin, the fix is a no-op.
    #[test]
    fn reserved_margin_fix_noop_when_consistent() {
        let mut storage = MockStorage::new();

        let bid = limit_order_with_reserved_raw(USER_A, 1, 5_000_000);
        plant_bid(&mut storage, eth_pair(), 2_000, 1, &bid);
        plant_user_state_with_reserved_raw(&mut storage, USER_A, 5_000_000);

        do_reserved_margin_rounding_error_fix(&mut storage).unwrap();

        let after = dango_perps::state::USER_STATES
            .load(&storage, USER_A)
            .unwrap();
        assert_eq!(after.reserved_margin, UsdValue::new_raw(5_000_000));
    }

    /// `do_referral_activated_referees_upgrade` rewrites legacy
    /// `UserReferralData` entries, adding `cumulative_global_active_referees`
    /// computed from referee stats.
    #[test]
    fn referral_activated_referees_migration() {
        let mut storage = MockStorage::new();

        let referrer: UserIndex = 1;
        let referee_a: UserIndex = 2;
        let referee_b: UserIndex = 3;
        let referee_c: UserIndex = 4;

        let day1 = Timestamp::from_seconds(86_400);
        let day2 = Timestamp::from_seconds(86_400 * 2);

        // Referee A has traded (last_day_active != 0).
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_a), &perps::RefereeStats {
                registered_at: day1,
                volume: UsdValue::new_int(1_000),
                commission_earned: UsdValue::new_int(10),
                last_day_active: day1,
            })
            .unwrap();

        // Referee B has traded.
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_b), &perps::RefereeStats {
                registered_at: day1,
                volume: UsdValue::new_int(500),
                commission_earned: UsdValue::new_int(5),
                last_day_active: day2,
            })
            .unwrap();

        // Referee C has NOT traded (last_day_active == 0).
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (referrer, referee_c), &perps::RefereeStats {
                registered_at: day2,
                ..Default::default()
            })
            .unwrap();

        // Seed two legacy USER_REFERRAL_DATA buckets for the referrer.
        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day1), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 2,
                referees_volume: UsdValue::new_int(1_000),
                commission_earned_from_referees: UsdValue::new_int(10),
                cumulative_active_referees: 1,
            })
            .unwrap();

        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day2), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 3,
                referees_volume: UsdValue::new_int(1_500),
                commission_earned_from_referees: UsdValue::new_int(15),
                cumulative_active_referees: 3,
            })
            .unwrap();

        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        // Both buckets should now have cumulative_global_active_referees = 2
        // (referee_a and referee_b traded, referee_c did not).
        let migrated_day1 = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day1))
            .unwrap();
        assert_eq!(migrated_day1.cumulative_global_active_referees, 2);
        assert_eq!(migrated_day1.cumulative_daily_active_referees, 1);
        assert_eq!(migrated_day1.referee_count, 2);

        let migrated_day2 = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day2))
            .unwrap();
        assert_eq!(migrated_day2.cumulative_global_active_referees, 2);
        assert_eq!(migrated_day2.cumulative_daily_active_referees, 3);
        assert_eq!(migrated_day2.referee_count, 3);
    }

    /// A referrer with no referees who have traded gets
    /// `cumulative_global_active_referees = 0` after the migration.
    #[test]
    fn referral_activated_referees_migration_no_active() {
        let mut storage = MockStorage::new();

        let referrer: UserIndex = 1;
        let day1 = Timestamp::from_seconds(86_400);

        legacy::USER_REFERRAL_DATA
            .save(&mut storage, (referrer, day1), &legacy::UserReferralData {
                volume: UsdValue::ZERO,
                commission_shared_by_referrer: UsdValue::ZERO,
                referee_count: 2,
                referees_volume: UsdValue::ZERO,
                commission_earned_from_referees: UsdValue::ZERO,
                cumulative_active_referees: 0,
            })
            .unwrap();

        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        let migrated = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (referrer, day1))
            .unwrap();
        assert_eq!(migrated.cumulative_global_active_referees, 0);
        assert_eq!(migrated.cumulative_daily_active_referees, 0);
    }

    /// `do_upgrade` chains all migrations: legacy `PairParam`s are
    /// rewritten, the global `Param` gains `max_action_batch_size`,
    /// legacy resting orders are rewritten, `NEXT_FILL_ID` is seeded,
    /// and drifted `reserved_margin` entries are repaired — all under
    /// one upgrade boundary.
    #[test]
    fn do_upgrade_runs_all_migrations() {
        let mut storage = MockStorage::new();

        // Seed legacy state for every migration.
        legacy::PAIR_PARAMS
            .save(&mut storage, &eth_pair(), &legacy_pair_param(1))
            .unwrap();
        legacy::PARAM.save(&mut storage, &legacy_param()).unwrap();
        legacy::BIDS
            .save(
                &mut storage,
                (eth_pair(), UsdPrice::new_int(2_000), Uint64::new(1)),
                &legacy_limit_order(USER_A, 10, 100),
            )
            .unwrap();

        // USER_A has one bid with reserved_margin = 100, but their
        // user_state claims 105 — a 5 USD orphan to be corrected.
        plant_user_state_with_reserved_raw(&mut storage, USER_A, 105_000_000);

        // Seed legacy referral data for the activated-referees migration.
        // User index 10 is a referrer with one active referee (index 11).
        dango_perps::state::REFERRER_TO_REFEREE_STATISTICS
            .save(&mut storage, (10, 11), &perps::RefereeStats {
                registered_at: Timestamp::from_seconds(86_400),
                volume: UsdValue::new_int(100),
                commission_earned: UsdValue::new_int(1),
                last_day_active: Timestamp::from_seconds(86_400),
            })
            .unwrap();
        legacy::USER_REFERRAL_DATA
            .save(
                &mut storage,
                (10, Timestamp::from_seconds(86_400)),
                &legacy::UserReferralData {
                    volume: UsdValue::ZERO,
                    commission_shared_by_referrer: UsdValue::ZERO,
                    referee_count: 1,
                    referees_volume: UsdValue::new_int(100),
                    commission_earned_from_referees: UsdValue::new_int(1),
                    cumulative_active_referees: 1,
                },
            )
            .unwrap();

        // Run all migrations sequentially (mirrors `do_upgrade`).
        do_pair_param_and_param_upgrades(&mut storage).unwrap();
        do_client_order_id_upgrade(&mut storage).unwrap();
        do_fill_id_upgrade(&mut storage).unwrap();
        do_reserved_margin_rounding_error_fix(&mut storage).unwrap();
        do_referral_activated_referees_upgrade(&mut storage).unwrap();

        // PairParam migrated.
        let pair = dango_perps::state::PAIR_PARAMS
            .load(&storage, &eth_pair())
            .unwrap();
        assert_eq!(
            pair.max_limit_price_deviation,
            MIGRATION_MAX_LIMIT_PRICE_DEVIATION
        );
        assert_eq!(pair.max_market_slippage, MIGRATION_MAX_MARKET_SLIPPAGE);
        assert_eq!(
            pair.funding_rate_multiplier,
            MIGRATION_FUNDING_RATE_MULTIPLIER
        );

        // Param migrated.
        assert_eq!(
            dango_perps::state::PARAM
                .load(&storage)
                .unwrap()
                .max_action_batch_size,
            MIGRATION_MAX_ACTION_BATCH_SIZE
        );

        // Order migrated.
        let (_key, order) = dango_perps::state::BIDS
            .idx
            .order_id
            .may_load(&storage, Uint64::new(1))
            .unwrap()
            .unwrap();
        assert_eq!(order.user, USER_A);
        assert_eq!(order.client_order_id, None);

        // NEXT_FILL_ID seeded.
        assert_eq!(
            dango_perps::state::NEXT_FILL_ID.load(&storage).unwrap(),
            FillId::ONE
        );

        // `reserved_margin` repaired: user_state now matches the sum of
        // the user's open orders (the one bid with reserved_margin = 100).
        let user_a_state = dango_perps::state::USER_STATES
            .load(&storage, USER_A)
            .unwrap();
        assert_eq!(user_a_state.reserved_margin, UsdValue::new_int(100));

        // Referral activated_referees migrated: referrer 10 has 1 active referee.
        let referral_data = dango_perps::state::USER_REFERRAL_DATA
            .load(&storage, (10, Timestamp::from_seconds(86_400)))
            .unwrap();
        assert_eq!(referral_data.cumulative_global_active_referees, 1);
        assert_eq!(referral_data.cumulative_daily_active_referees, 1);
    }
}
