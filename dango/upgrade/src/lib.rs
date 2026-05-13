use {
    dango_gateway::{RATE_LIMITS, SUPPLY_SNAPSHOTS},
    dango_order_book::{ASKS, BIDS, increase_liquidity_depths, may_invert_price},
    dango_perps::state::{PAIR_IDS, PAIR_PARAMS},
    dango_types::config::AppConfig,
    grug::{Addr, BlockInfo, Denom, JsonDeExt, Map, Order, StdResult, Storage, Uint128, addr},
    grug_app::{APP_CONFIG, AppResult, CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeSet,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Storage key the rate-limit-hardening release used for the per-denom
/// draining outbound cap. The rolling-window release replaces it with
/// `SUPPLY_SNAPSHOTS`, so the migration drops every entry behind this
/// prefix.
const OLD_OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");

/// Mirror of the bank contract's supply storage so the migration can read
/// the current supply per denom without going through the bank's query
/// path (which isn't available from the migration context).
const BANK_SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Find the address of the perps contract corresponding to the current chain.
    let chain_id = CHAIN_ID.load(&storage)?;
    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    // Look up the bank and gateway addresses for the gateway rolling-window
    // migration. Reading from on-chain config keeps the migration portable
    // across mainnet, testnet, and devnet without a per-chain address table.
    let chain_config = CONFIG.load(&storage)?;
    let app_config: AppConfig = APP_CONFIG.load(&storage)?.deserialize_json()?;
    let bank_address = chain_config.bank;
    let gateway_address = app_config.addresses.gateway;

    // `Storage` is `DynClone`, so `Box<dyn Storage>` is `Clone`. Each clone
    // points at the same backing KV store; writes through any provider land
    // in the same place.
    let mut gateway_storage =
        StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &gateway_address]);
    let bank_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &bank_address]);
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    do_bucket_size_backfill(&mut perps_storage)?;
    do_gateway_rolling_window_seed(&mut gateway_storage, &bank_storage)?;

    Ok(())
}

/// Drop the old draining-quota state and seed a supply snapshot for every
/// rate-limited denom so the rolling-window cap is enforced from the very
/// next withdraw, without waiting for the first cron tick after the
/// upgrade.
fn do_gateway_rolling_window_seed(
    gateway_storage: &mut dyn Storage,
    bank_storage: &dyn Storage,
) -> StdResult<()> {
    // Wipe every `OUTBOUND_QUOTAS` entry. The rolling-window contract
    // doesn't read this prefix; leaving it behind would just be dead state.
    OLD_OUTBOUND_QUOTAS.clear(gateway_storage, None, None);

    let mut seeded = 0usize;
    for denom in RATE_LIMITS.load(gateway_storage)?.keys() {
        let supply = BANK_SUPPLIES.load(bank_storage, denom)?;
        SUPPLY_SNAPSHOTS.save(gateway_storage, denom, &supply)?;
        seeded += 1;
    }

    tracing::info!("Seeded gateway supply snapshots for {seeded} denom(s)");

    Ok(())
}

/// Walk every pair's resting order book and populate `DEPTHS` for a new
/// bucket size equal to `tick_size`, then add `tick_size` to the pair's
/// `bucket_sizes`.
///
/// This recovers the original "smallest bucket equals tick" property of
/// the depth query: the tick was reduced 10× after deploy, but the
/// matching small bucket was never added because populating its depths
/// requires walking the entire book.
///
/// Idempotent: pairs whose `bucket_sizes` already contain `tick_size` are
/// skipped, so re-running is a no-op.
fn do_bucket_size_backfill(storage: &mut dyn Storage) -> StdResult<()> {
    let pair_ids = PAIR_IDS.load(storage)?;
    let mut backfilled = 0usize;

    for pair_id in pair_ids {
        let mut pair_param = PAIR_PARAMS.load(storage, &pair_id)?;

        // Skip pairs that already have `tick_size` as a configured bucket
        // size; their depths are maintained incrementally by the trade
        // path and need no migration.
        if pair_param.bucket_sizes.contains(&pair_param.tick_size) {
            continue;
        }

        // Only populate the *new* bucket size. Existing bucket sizes have
        // correct depths already; including them here would double-count.
        let new_buckets = BTreeSet::from([pair_param.tick_size]);

        // Bids store the inverted price (`!real_price`), so iteration in
        // ascending stored order visits them best-first. Un-invert before
        // passing the price to `increase_liquidity_depths`, which expects
        // the real (un-inverted) price.
        let bids = BIDS
            .prefix(pair_id.clone())
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for ((stored_price, _order_id), order) in bids {
            let real_price = may_invert_price(stored_price, true);
            let abs_size = order.size.checked_abs()?;
            increase_liquidity_depths(storage, &pair_id, true, real_price, abs_size, &new_buckets)?;
        }

        // Asks store the price as-is.
        let asks = ASKS
            .prefix(pair_id.clone())
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for ((stored_price, _order_id), order) in asks {
            let real_price = may_invert_price(stored_price, false);
            let abs_size = order.size.checked_abs()?;
            increase_liquidity_depths(
                storage,
                &pair_id,
                false,
                real_price,
                abs_size,
                &new_buckets,
            )?;
        }

        pair_param.bucket_sizes.insert(pair_param.tick_size);
        PAIR_PARAMS.save(storage, &pair_id, &pair_param)?;

        for bucket_size in &new_buckets {
            tracing::info!(
                %pair_id,
                %bucket_size,
                "Backfilled liquidity depth bucket"
            );
        }

        backfilled += 1;
    }

    tracing::info!("Backfilled liquidity depth buckets for {backfilled} pair(s)");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::do_bucket_size_backfill,
        dango_order_book::{
            ASKS, BIDS, DEPTHS, LimitOrder, PairId, Quantity, UsdPrice, UsdValue, may_invert_price,
        },
        dango_perps::state::{PAIR_IDS, PAIR_PARAMS},
        dango_types::perps::PairParam,
        grug::{Addr, Dec128_6, MockStorage, Order, StdResult, Storage, Timestamp, Uint64},
        std::{collections::BTreeSet, str::FromStr},
    };

    fn pid() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn p(s: &str) -> UsdPrice {
        UsdPrice::new(Dec128_6::from_str(s).unwrap())
    }

    fn usd(s: &str) -> UsdValue {
        UsdValue::new(Dec128_6::from_str(s).unwrap())
    }

    /// Save a resting bid at `real_price` with positive size `abs_size`.
    /// The price is inverted before insertion to match how the trade path
    /// stores bids (`!real_price`).
    fn save_bid(storage: &mut dyn Storage, real_price: UsdPrice, order_id: u64, abs_size: i128) {
        let order = LimitOrder {
            user: Addr::mock(1),
            size: Quantity::new_int(abs_size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(0),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
        };
        let key = (
            pid(),
            may_invert_price(real_price, true),
            Uint64::new(order_id),
        );
        BIDS.save(storage, key, &order).unwrap();
    }

    /// Save a resting ask at `real_price` with positive size `abs_size`. The
    /// stored size is the negation, since `LimitOrder.size` is signed.
    fn save_ask(storage: &mut dyn Storage, real_price: UsdPrice, order_id: u64, abs_size: i128) {
        let order = LimitOrder {
            user: Addr::mock(1),
            size: Quantity::new_int(-abs_size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(0),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
        };
        let key = (pid(), real_price, Uint64::new(order_id));
        ASKS.save(storage, key, &order).unwrap();
    }

    /// Migration walks the book, populates `DEPTHS` for `tick_size`,
    /// leaves existing-bucket `DEPTHS` untouched, and adds `tick_size` to
    /// `PairParam.bucket_sizes`.
    #[test]
    fn bucket_size_backfill_populates_depths() {
        let mut storage = MockStorage::new();
        let pair_id = pid();

        // tick_size = 0.001, existing buckets = {0.01, 0.1, 1}.
        let pair_param = PairParam {
            tick_size: p("0.001"),
            bucket_sizes: BTreeSet::from([p("0.01"), p("0.1"), p("1")]),
            ..Default::default()
        };
        PAIR_IDS
            .save(&mut storage, &BTreeSet::from([pair_id.clone()]))
            .unwrap();
        PAIR_PARAMS
            .save(&mut storage, &pair_id, &pair_param)
            .unwrap();

        // Two bids at 100.000 (will aggregate) and one at 99.999.
        save_bid(&mut storage, p("100.000"), 1, 5);
        save_bid(&mut storage, p("100.000"), 2, 3);
        save_bid(&mut storage, p("99.999"), 3, 2);

        // Two asks at 101.000 (will aggregate) and one at 101.001.
        save_ask(&mut storage, p("101.000"), 4, 4);
        save_ask(&mut storage, p("101.000"), 5, 6);
        save_ask(&mut storage, p("101.001"), 6, 7);

        // Pre-seed an existing-bucket entry (bucket_size 0.1, bid bucket
        // 100.0) to verify the migration only touches the new bucket size.
        let preseeded = (Quantity::new_int(999), UsdValue::new_int(99_900));
        DEPTHS
            .save(
                &mut storage,
                (&pair_id, p("0.1"), true, p("100")),
                &preseeded,
            )
            .unwrap();

        do_bucket_size_backfill(&mut storage).unwrap();

        // Bid 100.000: aggregated size 5 + 3 = 8, notional = 8 * 100 = 800.
        let entry = DEPTHS
            .load(&storage, (&pair_id, p("0.001"), true, p("100")))
            .unwrap();
        assert_eq!(entry, (Quantity::new_int(8), usd("800")));

        // Bid 99.999: size 2, notional = 2 * 99.999 = 199.998.
        let entry = DEPTHS
            .load(&storage, (&pair_id, p("0.001"), true, p("99.999")))
            .unwrap();
        assert_eq!(entry, (Quantity::new_int(2), usd("199.998")));

        // Ask 101.000: aggregated size 4 + 6 = 10, notional = 10 * 101 = 1010.
        let entry = DEPTHS
            .load(&storage, (&pair_id, p("0.001"), false, p("101")))
            .unwrap();
        assert_eq!(entry, (Quantity::new_int(10), usd("1010")));

        // Ask 101.001: size 7, notional = 7 * 101.001 = 707.007.
        let entry = DEPTHS
            .load(&storage, (&pair_id, p("0.001"), false, p("101.001")))
            .unwrap();
        assert_eq!(entry, (Quantity::new_int(7), usd("707.007")));

        // Pre-seeded existing-bucket entry is preserved.
        let preserved = DEPTHS
            .load(&storage, (&pair_id, p("0.1"), true, p("100")))
            .unwrap();
        assert_eq!(preserved, preseeded);

        // bucket_sizes now contains tick_size.
        let updated = PAIR_PARAMS.load(&storage, &pair_id).unwrap();
        assert_eq!(
            updated.bucket_sizes,
            BTreeSet::from([p("0.001"), p("0.01"), p("0.1"), p("1")])
        );
    }

    /// If `tick_size` is already a configured bucket, the migration must
    /// not walk the book or write to `DEPTHS`.
    #[test]
    fn bucket_size_backfill_skips_when_tick_already_a_bucket() {
        let mut storage = MockStorage::new();
        let pair_id = pid();

        let pair_param = PairParam {
            tick_size: p("0.1"),
            bucket_sizes: BTreeSet::from([p("0.1"), p("1"), p("10")]),
            ..Default::default()
        };
        PAIR_IDS
            .save(&mut storage, &BTreeSet::from([pair_id.clone()]))
            .unwrap();
        PAIR_PARAMS
            .save(&mut storage, &pair_id, &pair_param)
            .unwrap();

        // Pre-seed one DEPTHS entry to anchor the assertion.
        let preseeded = (Quantity::new_int(123), UsdValue::new_int(456));
        DEPTHS
            .save(
                &mut storage,
                (&pair_id, p("0.1"), true, p("100")),
                &preseeded,
            )
            .unwrap();

        // A bid that, in production, would have populated DEPTHS for every
        // configured bucket. Here we add it directly to BIDS without the
        // accompanying DEPTHS write to verify the migration leaves DEPTHS
        // alone when the pair is skipped.
        save_bid(&mut storage, p("100"), 1, 5);

        do_bucket_size_backfill(&mut storage).unwrap();

        let updated = PAIR_PARAMS.load(&storage, &pair_id).unwrap();
        assert_eq!(
            updated.bucket_sizes,
            BTreeSet::from([p("0.1"), p("1"), p("10")])
        );

        let preserved = DEPTHS
            .load(&storage, (&pair_id, p("0.1"), true, p("100")))
            .unwrap();
        assert_eq!(preserved, preseeded);

        let count = DEPTHS.range(&storage, None, None, Order::Ascending).count();
        assert_eq!(
            count, 1,
            "migration should not touch DEPTHS for a skipped pair"
        );
    }

    /// Running the migration twice yields the same `DEPTHS` as running
    /// once: the second pass short-circuits via the `contains` check.
    #[test]
    fn bucket_size_backfill_idempotent() {
        let mut storage = MockStorage::new();
        let pair_id = pid();

        let pair_param = PairParam {
            tick_size: p("0.001"),
            bucket_sizes: BTreeSet::from([p("0.01"), p("0.1"), p("1")]),
            ..Default::default()
        };
        PAIR_IDS
            .save(&mut storage, &BTreeSet::from([pair_id.clone()]))
            .unwrap();
        PAIR_PARAMS
            .save(&mut storage, &pair_id, &pair_param)
            .unwrap();

        save_bid(&mut storage, p("100"), 1, 5);
        save_ask(&mut storage, p("101"), 2, 4);

        do_bucket_size_backfill(&mut storage).unwrap();

        let after_first = DEPTHS
            .range(&storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        do_bucket_size_backfill(&mut storage).unwrap();

        let after_second = DEPTHS
            .range(&storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(after_first, after_second);
    }

    mod gateway_rolling_window {
        use {
            super::super::{BANK_SUPPLIES, OLD_OUTBOUND_QUOTAS, do_gateway_rolling_window_seed},
            dango_gateway::{RATE_LIMITS, SUPPLY_SNAPSHOTS},
            dango_types::gateway::RateLimit,
            grug::{Denom, MockStorage, NumberConst, Udec128, Uint128, btree_map},
            std::str::FromStr,
        };

        fn denom(s: &str) -> Denom {
            Denom::from_str(s).unwrap()
        }

        #[test]
        fn seeds_snapshots_and_wipes_old_quotas() {
            let mut gateway = MockStorage::new();
            let mut bank = MockStorage::new();

            let usdc = denom("bridge/usdc");
            let eth = denom("bridge/eth");

            // Pre-upgrade state: configured rate limits plus drained
            // OUTBOUND_QUOTAS that the rolling-window release shouldn't read.
            RATE_LIMITS
                .save(&mut gateway, &btree_map! {
                    usdc.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
                    eth.clone()  => RateLimit::new_unchecked(Udec128::new_percent(20)),
                })
                .unwrap();

            OLD_OUTBOUND_QUOTAS
                .save(&mut gateway, &usdc, &Uint128::new(123))
                .unwrap();
            OLD_OUTBOUND_QUOTAS
                .save(&mut gateway, &eth, &Uint128::new(456))
                .unwrap();

            // Bank state mirrored from the bank contract's `supply` map.
            BANK_SUPPLIES
                .save(&mut bank, &usdc, &Uint128::new(100_000_000))
                .unwrap();
            BANK_SUPPLIES
                .save(&mut bank, &eth, &Uint128::new(50_000_000))
                .unwrap();

            do_gateway_rolling_window_seed(&mut gateway, &bank).unwrap();

            // Snapshots match the bank's current supply for every
            // rate-limited denom.
            assert_eq!(
                SUPPLY_SNAPSHOTS.load(&gateway, &usdc).unwrap(),
                Uint128::new(100_000_000),
            );
            assert_eq!(
                SUPPLY_SNAPSHOTS.load(&gateway, &eth).unwrap(),
                Uint128::new(50_000_000),
            );

            // The legacy OUTBOUND_QUOTAS map is empty.
            assert!(
                OLD_OUTBOUND_QUOTAS
                    .may_load(&gateway, &usdc)
                    .unwrap()
                    .is_none()
            );
            assert!(
                OLD_OUTBOUND_QUOTAS
                    .may_load(&gateway, &eth)
                    .unwrap()
                    .is_none()
            );
        }

        /// A chain with no rate limits at all is a no-op: nothing reads,
        /// nothing writes, and OUTBOUND_QUOTAS stays empty.
        #[test]
        fn empty_rate_limits_is_noop() {
            let mut gateway = MockStorage::new();
            let bank = MockStorage::new();

            RATE_LIMITS.save(&mut gateway, &btree_map! {}).unwrap();

            do_gateway_rolling_window_seed(&mut gateway, &bank).unwrap();

            assert_eq!(
                SUPPLY_SNAPSHOTS
                    .range(&gateway, None, None, grug::Order::Ascending)
                    .count(),
                0,
            );
        }
    }
}
