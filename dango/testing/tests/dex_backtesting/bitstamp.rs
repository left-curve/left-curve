#![cfg(test)]

use {
    crate::data_adapters::bitstamp_data_adapter::{
        BitstampDataAdapter, DexOrderConfig, OrderBatch,
    },
    dango_genesis::{DexOption, GenesisOption},
    dango_testing::{BridgeOp, Preset, TestOption, setup_test_naive_with_custom_genesis},
    dango_types::{
        constants::{dango, usdc},
        dex::{
            self, AmountOption, AvellanedaStoikovParams, CancelOrderRequest, CreateOrderRequest,
            Geometric, OrderCreated, OrderFilled, OrderId, PairParams, PairUpdate,
            PassiveLiquidity,
        },
        gateway::Remote,
        oracle::{self, PriceSource},
    },
    grug::{
        Addr, Addressable, Bounded, CheckedContractEvent, Coin, Coins, Dec, Denom, Duration, Inner,
        JsonDeExt, MultiplyFraction, Number, NumberConst, ResultExt, SearchEvent, Signer,
        TestSuite, Timestamp, Udec128, Uint128, btree_map, btree_set, coins,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        path::PathBuf,
        str::FromStr,
    },
};

use test_case::test_case;

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
            gamma: Dec::from_str("0.1").unwrap(),
            time_horizon: Duration::from_seconds(120),
            k: Dec::from_str("1.3").unwrap(),
            half_life: Duration::from_seconds(30),
            base_inventory_target_percentage: Bounded::new(
                Udec128::new_percent(50),
            )
            .unwrap(),
        },
    },
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/bitstamp/bitstamp_btcusd_20251118_132040.csv"),
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dex/market_data/bitstamp/pyth_btcusd_20251118_132040.csv"),
    Duration::from_millis(1000),
    Duration::from_millis(500),
    8,
    6,
    1000
)]
fn test_replay_bitstamp_orders_on_dex(
    pool_params: Geometric,
    order_data_path: PathBuf,
    price_data_path: PathBuf,
    block_time: Duration,
    max_oracle_staleness: Duration,
    base_precision: u8,
    quote_precision: u8,
    blocks_to_replay: usize,
) {
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
                    // BridgeOp {
                    //     remote: Remote::Bitcoin,
                    //     amount: Uint128::new(100_000_000_000_000),
                    //     recipient: accounts.user1.address(),
                    // },
                    // BridgeOp {
                    //     remote: Remote::Bitcoin,
                    //     amount: Uint128::new(100_000_000_000_000),
                    //     recipient: accounts.owner.address(),
                    // },
                ]
            },
            block_time: Duration::ZERO,
            ..TestOption::default()
        },
        GenesisOption {
            dex: DexOption {
                cron_frequency: Duration::from_millis(1), /* Set to >0 so that cron is not executed immediately if block time is not moved forward */
                pairs: vec![PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Geometric(pool_params.clone()),
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

    eprintln!("Loading market data...");

    // Configure order conversion
    let order_config = DexOrderConfig {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        base_amount_scale: 10u128.pow(base_precision as u32),
        quote_amount_scale: 10u128.pow(quote_precision as u32),
        passive_orders_per_side: pool_params.limit,
    };

    // Load market data
    let mut adapter =
        BitstampDataAdapter::from_csv(&order_data_path, &price_data_path, order_config).unwrap();

    println!(
        "adapter start timestamp: {:#?}",
        adapter.current_timestamp().into_seconds()
    );

    eprintln!("Loaded {} orders from CSV", adapter.total_orders());

    // Get a trader account
    let user1 = &mut accounts.user1;

    eprintln!("Starting order replay...");

    // Peek first oracle price before starting iteration
    let first_oracle_price = adapter.first_oracle_price().unwrap();

    println!("first_oracle_price: {:#?}", first_oracle_price);

    // Register oracle price sources for DANGO and USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: first_oracle_price.price,
                    precision: base_precision,
                    timestamp: first_oracle_price.timestamp,
                },
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: quote_precision,
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
                dango::DENOM.clone() => Uint128::new(1_000_000 * 10u128.pow(base_precision as u32)),
                usdc::DENOM.clone() => Uint128::new(1_000_000)
                    .checked_mul_dec(
                        first_oracle_price
                            .price
                            .checked_mul(Udec128::new(10u128.pow(quote_precision as u32)))
                            .unwrap()
                    )
                    .unwrap(),
            },
        )
        .should_succeed();

    println!("Provided liquidity");

    // Record dex balances
    suite.balances().record(&contracts.dex);

    // Replay orders in batches using the iterator
    // Process batches one at a time to avoid borrow checker issues when removing filled orders
    let mut cleared_orders: BTreeSet<OrderId> = BTreeSet::new();
    let mut bitstamp_to_dex_order_id_mapping: BTreeMap<u64, OrderId> = BTreeMap::new();
    for (
        block_count,
        OrderBatch {
            creates,
            cancels,
            changes,
            oracle_price,
            block_time,
        },
    ) in adapter
        .batches(block_time, max_oracle_staleness, pool_params.limit as u64 * 2 + 1) // 2 is the initial order ID value because two passive orders per side are created when we provide liquidity
        .take(blocks_to_replay)
        .enumerate()
    {
        println!(
            "Block {}: {}",
            block_count,
            oracle_price.timestamp.into_seconds()
        );

        // Log oracle price
        println!(
            "  Oracle price: {} (confidence: {}, expo: {})",
            oracle_price.price, oracle_price.confidence, oracle_price.expo
        );

        // Update fixed oracle price.
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                    dango::DENOM.clone() => PriceSource::Fixed {
                        humanized_price: oracle_price.price,
                        precision: base_precision,
                        timestamp: Timestamp::from_seconds(1730802926),
                    },
                }),
                Coins::new(),
            )
            .should_succeed();

        if creates.is_empty() && cancels.is_empty() {
            continue;
        }

        println!(
            "{} create orders, {} cancel orders, {} change orders",
            creates.len(),
            cancels.len(),
            changes.len()
        );

        // First we create all the orders
        for (bitstamp_order_id, create_req) in creates {
            create_order(
                &mut suite,
                user1,
                contracts.dex.address(),
                bitstamp_order_id,
                create_req,
                &mut bitstamp_to_dex_order_id_mapping,
            );
        }

        // Now we handle all cancellations
        for bitstamp_order_id in cancels {
            cancel_order(
                &mut suite,
                user1,
                contracts.dex.address(),
                &mut bitstamp_to_dex_order_id_mapping,
                &cleared_orders,
                bitstamp_order_id,
            );
        }

        // Now we handle all updated orders by first cancelling the old order and then creating the new one
        for (bitstamp_order_id, create_req) in changes {
            cancel_order(
                &mut suite,
                user1,
                contracts.dex.address(),
                &mut bitstamp_to_dex_order_id_mapping,
                &cleared_orders,
                bitstamp_order_id,
            );
            create_order(
                &mut suite,
                user1,
                contracts.dex.address(),
                bitstamp_order_id,
                create_req,
                &mut bitstamp_to_dex_order_id_mapping,
            );
        }

        // Fast forward by the duration of the block and make a new block
        // This will trigger cron execution and fill orders.
        println!("increasing time by {:?} ms", block_time.into_millis());
        let block_outcome = suite.increase_time(block_time).block_outcome;

        // Ensure all transactions were successful
        block_outcome.tx_outcomes.iter().for_each(|tx_outcome| {
            tx_outcome.result.clone().should_succeed();
        });

        // Extract the OrderFilled events from the cron outcome
        let cron_outcome = block_outcome.cron_outcomes.first().unwrap();
        let order_filled_events = cron_outcome
            .clone()
            .search_event::<CheckedContractEvent>()
            .with_predicate(move |event| {
                event.contract == contracts.dex.address() && event.ty == "order_filled"
            })
            .take()
            .all()
            .into_iter()
            .map(|e| e.event.data.deserialize_json::<OrderFilled>().unwrap())
            .collect::<Vec<_>>();

        // Update the filled orders set with all the cleared orders
        for event in order_filled_events {
            if event.cleared {
                println!("Order cleared with id: {}", event.id);
                cleared_orders.insert(event.id);
            }
        }
    }

    // Assert that the dex balance has increased
    let balance_changes = suite.balances().changes(&contracts.dex);
    let base_balance_change = balance_changes.get(&dango::DENOM.clone()).unwrap();
    let quote_balance_change = balance_changes.get(&usdc::DENOM.clone()).unwrap();
    println!("base_balance_change: {:#?}", base_balance_change);
    println!("quote_balance_change: {:#?}", quote_balance_change);
    println!(
        "adapter end timestamp: {:#?}",
        adapter.current_timestamp().into_seconds()
    );

    println!(
        "Processed {}/{} total orders from CSV",
        adapter.processed_orders(),
        adapter.total_orders()
    );
}

fn create_order(
    suite: &mut TestSuite,
    user1: &mut dyn Signer,
    dex_contract: Addr,
    bitstamp_order_id: u64,
    create_req: CreateOrderRequest,
    bitstamp_to_dex_order_id_mapping: &mut BTreeMap<u64, OrderId>,
) {
    let funds = funds_required(&vec![create_req.clone()]);
    let tx_success = suite
        .execute(
            user1,
            dex_contract,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![create_req],
                cancels: None,
            },
            funds,
        )
        .should_succeed();

    // Extract the OrderCreated event
    let order_created_events = tx_success
        .events
        .search_event::<CheckedContractEvent>()
        .with_predicate(move |event| event.contract == dex_contract && event.ty == "order_created")
        .take()
        .all()
        .into_iter()
        .map(|e| e.event.data.deserialize_json::<OrderCreated>().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(order_created_events.len(), 1);

    let order_created_event = order_created_events.first().unwrap();
    bitstamp_to_dex_order_id_mapping.insert(bitstamp_order_id, order_created_event.id);
}

fn cancel_order(
    suite: &mut TestSuite,
    user1: &mut dyn Signer,
    dex_contract: Addr,
    bitstamp_to_dex_order_id: &mut BTreeMap<u64, OrderId>,
    filled_orders: &BTreeSet<OrderId>,
    bitstamp_order_id: u64,
) {
    if !bitstamp_to_dex_order_id.contains_key(&bitstamp_order_id) {
        return;
    }

    let dex_order_id = bitstamp_to_dex_order_id.get(&bitstamp_order_id).unwrap();

    if filled_orders.contains(dex_order_id) {
        return;
    }

    suite
        .execute(
            user1,
            dex_contract,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![],
                cancels: Some(CancelOrderRequest::Some(BTreeSet::from([*dex_order_id]))),
            },
            Coins::new(),
        )
        .should_succeed();
}
