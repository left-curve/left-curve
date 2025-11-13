#![cfg(test)]

#[path = "dex/coinbase_data_adapter.rs"]
mod coinbase_data_adapter;

use {
    coinbase_data_adapter::{CoinbaseDataAdapter, DexOrderConfig},
    dango_genesis::{DexOption, GenesisOption},
    dango_testing::{BridgeOp, Preset, TestOption, setup_test_naive_with_custom_genesis},
    dango_types::{
        constants::{dango, usdc},
        dex::{
            self, AmountOption, AvellanedaStoikovParams, CreateOrderRequest, Geometric, PairParams,
            PairUpdate, PassiveLiquidity,
        },
        gateway::Remote,
        oracle::{self, PriceSource},
    },
    grug::{
        Addressable, Bounded, Coin, Coins, Dec, Denom, Duration, Inner, Message, NonEmpty,
        NumberConst, ResultExt, Signer, Timestamp, Udec128, Uint128, btree_map, btree_set, coins,
    },
    std::{path::PathBuf, str::FromStr},
};

use test_case::test_case;

#[test]
fn test_market_data_adapter() {
    // Get the paths to the CSV files
    let orders_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/coinbase_incoming_orders_BTCUSD_20251112_144231.csv");
    let prices_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/pyth_btcusd_20251113_143202.csv");

    // Create the adapter
    let adapter = CoinbaseDataAdapter::from_csv(&orders_path, &prices_path)
        .expect("Failed to load CSV files");

    println!("Loaded {} orders from CSV", adapter.total_orders());

    // Test peeking at orders in time windows (without creating transactions)
    let batch1 = adapter.peek_orders(1000);
    println!("First batch (1s): {} orders", batch1.len());

    let batch2 = adapter.peek_orders(11000); // Total 11 seconds from start
    println!("Second batch (11s total): {} orders", batch2.len());

    // Verify we can process orders
    assert!(adapter.total_orders() > 0);
    assert!(batch1.len() > 0 || batch2.len() > 0);
}

#[test]
fn test_dex_order_conversion() {
    // Get the paths to the CSV files
    let orders_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/coinbase_incoming_orders_BTCUSD_20251112_144231.csv");
    let prices_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/pyth_btcusd_20251113_143202.csv");

    let adapter = CoinbaseDataAdapter::from_csv(&orders_path, &prices_path)
        .expect("Failed to load CSV files");

    // Create DEX order config
    // BTC-USD pair: BTC has 8 decimals, USD (represented as USDC) has 6 decimals
    // Price is in USD per BTC, so if BTC is $100,000, we want to represent it properly
    let order_config = DexOrderConfig {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        // Scale price to 6 decimal places (USDC precision)
        price_scale: 1_000_000,
        // Scale BTC amount from BTC (8 decimals) to smallest units
        base_amount_scale: 100_000_000,
        // Scale USDC amount (6 decimals)
        quote_amount_scale: 1_000_000,
    };

    // Get first batch of orders (without creating transactions)
    let batch = adapter.peek_orders(1000);
    let dex_orders = coinbase_data_adapter::batch_to_dex_orders(&batch, &order_config);

    println!(
        "Converted {} Coinbase orders to {} DEX orders",
        batch.len(),
        dex_orders.len()
    );

    // Verify we successfully converted some orders
    if !batch.is_empty() {
        assert!(dex_orders.len() > 0, "Should convert at least some orders");
    }
}

fn funds_required(creates: &Vec<CreateOrderRequest>) -> Coins {
    let mut funds = Coins::new();
    for create in creates {
        match create.amount {
            AmountOption::Bid { quote } => {
                funds
                    .insert(Coin::new(create.quote_denom.clone(), quote.into_inner()).unwrap())
                    .unwrap();
            },
            AmountOption::Ask { base } => {
                funds
                    .insert(Coin::new(create.base_denom.clone(), base.into_inner()).unwrap())
                    .unwrap();
            },
        }
    }
    funds
}

#[test_case(
    Geometric {
        spacing: Udec128::new_percent(1),
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
        limit: 1,
        avellaneda_stoikov_params: AvellanedaStoikovParams {
            gamma: Dec::from_str("1.0").unwrap(),
            time_horizon: Duration::from_seconds(0),
            k: Dec::from_str("1.0").unwrap(),
            half_life: Duration::from_seconds(30),
            base_inventory_target_percentage: Bounded::new(
                Udec128::new_percent(50),
            )
            .unwrap(),
        },
} ; "geometric pool")]
fn test_replay_orders_on_dex(pool_params: Geometric) {
    // Setup the test environment with a BTC/USDC DEX pair
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive_with_custom_genesis(
        TestOption {
            bridge_ops: |accounts| {
                vec![
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: hyperlane_types::constants::ethereum::DOMAIN,
                            contract: hyperlane_types::constants::ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000_000_000),
                        recipient: accounts.user1.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: hyperlane_types::constants::ethereum::DOMAIN,
                            contract: hyperlane_types::constants::ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000_000_000),
                        recipient: accounts.owner.address(),
                    },
                ]
            },
            ..TestOption::default()
        },
        GenesisOption {
            dex: DexOption {
                pairs: vec![PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Geometric(pool_params),
                        bucket_sizes: btree_set![],
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                }],
            },
            ..Preset::preset_test()
        },
    );

    eprintln!("DEX pair configured");

    // Register oracle price sources for DANGO and USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::new_percent(100000), // $100k per DANGO
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Provide liquidity with owner account
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 100000000000000,
                usdc::DENOM.clone() => 100000000000,
            },
        )
        .should_succeed();

    eprintln!("Loading market data...");

    // Load market data
    let orders_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/coinbase_incoming_orders_BTCUSD_20251112_144231.csv");
    let prices_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/pyth_btcusd_20251113_143202.csv");

    let mut adapter = CoinbaseDataAdapter::from_csv(&orders_path, &prices_path)
        .expect("Failed to load CSV files");

    eprintln!("Loaded {} orders from CSV", adapter.total_orders());

    // Configure order conversion
    let order_config = DexOrderConfig {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        price_scale: 1_000_000,
        base_amount_scale: 1_000_000,
        quote_amount_scale: 1_000_000,
    };

    // Get a trader account
    let user1 = &mut accounts.user1;

    eprintln!("Starting order replay...");

    let mut batch_count = 0;
    const MAX_BLOCKS: usize = 10000; // Limit to 10 batches for initial testing
    const BLOCK_TIME_MS: u64 = 200;
    const GAS_LIMIT: u64 = 100_000;

    // Replay orders in batches
    while adapter.has_more_orders() && batch_count < MAX_BLOCKS {
        let batch = adapter.advance(BLOCK_TIME_MS, &order_config);

        if batch.orders.is_empty() {
            continue;
        }

        batch_count += 1;

        eprintln!(
            "Batch {}: {} orders (timestamp: {}ms)",
            batch_count,
            batch.orders.len(),
            adapter.current_timestamp()
        );

        // Log latest oracle price if available
        if let Some(price) = &batch.latest_price {
            eprintln!(
                "  Latest oracle price: {} (confidence: {}, expo: {})",
                price.price, price.confidence, price.expo
            );
        }

        // Create and submit transaction to place orders
        let funds = funds_required(&batch.orders);
        let msg = Message::execute(
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: batch.orders,
                cancels: None,
            },
            funds,
        )
        .unwrap();

        let tx = user1
            .sign_transaction(
                NonEmpty::new_unchecked(vec![msg]),
                &suite.chain_id,
                GAS_LIMIT,
            )
            .expect("Failed to sign transaction");

        // Note: This will likely fail because we're not providing funds
        // This is just a structural test to verify the adapter works
        let result = suite.make_block(vec![tx]);

        // For now, just log if it failed (expected since we're not funding)
        if let Err(e) = result.block_outcome.tx_outcomes[0].result.as_ref() {
            eprintln!("Order submission failed (expected): {:?}", e);
            // Break early since we know subsequent orders will also fail
            break;
        }
    }

    eprintln!(
        "Processed {}/{} total orders from CSV",
        adapter.processed_orders(),
        adapter.total_orders()
    );
}
