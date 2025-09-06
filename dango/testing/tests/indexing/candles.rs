use {
    assertor::*,
    chrono::{DateTime, TimeDelta},
    dango_genesis::Contracts,
    dango_indexer_clickhouse::entities::{
        CandleInterval, candle::Candle, candle_query::CandleQueryBuilder, pair_price::PairPrice,
        pair_price_query::PairPriceQueryBuilder,
    },
    dango_testing::{
        TestAccounts, TestOption, TestSuite, TestSuiteWithIndexer,
        constants::MOCK_GENESIS_TIMESTAMP, create_limit_order_request, setup_test_with_indexer,
    },
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        BlockInfo, Coins, Duration, Hash256, Int, Message, MultiplyFraction, NonEmpty, NonZero,
        NumberConst, ResultExt, Signer, StdResult, Timestamp, Udec128, Udec128_6, Udec128_24,
        Uint128, btree_map, coins,
    },
    grug_app::Indexer,
    std::str::FromStr,
};

#[ignore = "This test is now hanging, should be fixed, the mock feature is not working"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_mocked_clickhouse() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default().with_mocked_clickhouse()).await;

    let recording = clickhouse_context
        .mock()
        .add(clickhouse::test::handlers::record());

    // NOTE: used the same code as `dex_works` in `dex.rs`

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    let orders_to_submit: Vec<(Direction, u128, u128)> = vec![
        (Direction::Bid, 30, 25), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask, 5, 10),  //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - 50% filled
    ];

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128_24::new(price);
            let amount = Uint128::new(amount);

            let (funds, request) = create_limit_order_request(
                dango::DENOM.clone(),
                usdc::DENOM.clone(),
                direction,
                amount,
                price,
            );

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![request],
                    cancels: None,
                },
                funds,
            )?;

            signer.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    suite.app.indexer.wait_for_finish()?;

    let clickhouse_inserts = recording.collect::<Vec<PairPrice>>().await;

    assert_that!(clickhouse_inserts).has_length(1);

    let pair_price = clickhouse_inserts[0].clone();

    // Manual asserts so if clearing price changes, it doesn't break this test.
    assert_that!(pair_price.quote_denom).is_equal_to("bridge/usdc".to_string());
    assert_that!(pair_price.base_denom).is_equal_to("dango".to_string());
    assert_that!(pair_price.clearing_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let pair_price_query_builder =
        PairPriceQueryBuilder::new("dango".to_string(), "bridge/usdc".to_string()).with_limit(1);

    let pair_price = pair_price_query_builder
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .unwrap();

    // Manual asserts so if clearing price changes, it doesn't break this test.
    assert_that!(pair_price.quote_denom).is_equal_to("bridge/usdc".to_string());
    assert_that!(pair_price.base_denom).is_equal_to("dango".to_string());
    assert_that!(pair_price.clearing_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);
    assert_that!(pair_price.volume_base)
        .is_equal_to::<Udec128_6>(Udec128_6::from_str("25.0").unwrap());
    assert_that!(pair_price.volume_quote)
        .is_equal_to::<Udec128_6>(Udec128_6::from_str("687.5").unwrap());

    // Makes sure we get correct precision: 27.4 without specific number, since this can change.
    assert_that!(pair_price.clearing_price.to_string().len()).is_equal_to(4);

    let candle_query_builder = CandleQueryBuilder::new(
        CandleInterval::OneMinute,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    )
    .with_limit(1);

    let candle_1m = candle_query_builder
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .unwrap();

    assert_that!(candle_1m.quote_denom).is_equal_to("bridge/usdc".to_string());
    assert_that!(candle_1m.base_denom).is_equal_to("dango".to_string());
    assert_that!(candle_1m.open).is_equal_to(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1m.high).is_equal_to(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1m.low).is_equal_to(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1m.close).is_equal_to(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1m.volume_base).is_equal_to(Udec128_6::from_str("25.0").unwrap());
    assert_that!(candle_1m.volume_quote).is_equal_to(Udec128_6::from_str("687.5").unwrap());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse_and_one_minute_interval() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default()).await;

    for _ in 0..10 {
        create_pair_prices(&mut suite, &mut accounts, &contracts).await?;
    }

    suite.app.indexer.wait_for_finish()?;

    let pair_prices = PairPriceQueryBuilder::new("dango".to_string(), "bridge/usdc".to_string())
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?
        .pair_prices;

    assert_that!(pair_prices.clone().len()).is_at_least(10);

    let candles_1m = CandleQueryBuilder::new(
        CandleInterval::OneMinute,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    )
    .fetch_all(clickhouse_context.clickhouse_client())
    .await?;

    assert_that!(candles_1m.candles).has_length(1);
    assert_that!(candles_1m.has_next_page).is_false();
    assert_that!(candles_1m.has_previous_page).is_false();

    let candle = &candles_1m.candles[0];

    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1971-01-01T00:00:00Z")
            .unwrap()
            .naive_utc(),
    );

    assert_that!(candle.open).is_equal_to(pair_prices.last().unwrap().clearing_price);

    assert_that!(candle.high)
        .is_equal_to(pair_prices.iter().map(|p| p.clearing_price).max().unwrap());

    assert_that!(candle.low)
        .is_equal_to(pair_prices.iter().map(|p| p.clearing_price).min().unwrap());

    assert_that!(candle.close).is_equal_to(pair_prices.first().unwrap().clearing_price);

    assert_that!(candle.volume_base)
        .is_equal_to(pair_prices.iter().map(|p| p.volume_base).sum::<Udec128_6>());

    assert_that!(candle.volume_quote).is_equal_to(
        pair_prices
            .iter()
            .map(|p| p.volume_quote)
            .sum::<Udec128_6>(),
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse_and_one_second_interval() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default()).await;

    // Call the `create_pair_prices` function 10 times.
    //
    // The orders placed are taken from this example:
    // https://75m6j-xiaaa-aaaap-ahq4q-cai.icp0.io/?bids=30%2C25%3B20%2C10%3B10%2C10&asks=5%2C10%3B15%2C10%3B25%2C10
    // It will find the price range 25--30 maximizes the trading volume (denoted
    // in the base asset).
    //
    // According to our algorithm, the first time this is done, as no previous
    // auction exists, the clearing price is chosen at the middle point of the
    // range, which is 27.5. The trading volume is 25 units of base asset, or
    // 25 * 27.5 = 687.5 units of quote asset.
    //
    // After the auction, the resting order book state is as follows:
    // https://75m6j-xiaaa-aaaap-ahq4q-cai.icp0.io/?bids=20%2C10%3B10%2C10&asks=25%2C5
    // - best bid price: 20
    // - best ask price: 25
    // - mid price: 22.5
    //
    // The function is then called 9 more times. Now, since the mid price exists
    // and is smaller than the lower bound of the range (22.5 < 25), the clearing
    // price is chosen as 25 each time. Volume in base asset: 25; volume in
    // quote asset: 25 * 25 = 625.
    //
    // Summary:
    // - Candle 0s: contains blocks 1-3; block times: 0.25, 0.50, 0.75;
    //   open 27.5, high 27.5, low 25, close 25;
    //   volume base: 25 * 3 = 75; volume quote: 687.5 + 625 * 2 = 1937.5.
    // - Candle 1s: contains blocks 4-7; block times: 1.00, 1.25, 1.50, 1.75;
    //   open 25, high 25, low 25, close 25;
    //   volume base: 25 * 4 = 100; volume quote: 625 * 4 = 2500.
    // - Candle 2s: contains blocks 8-10; block times: 2.00, 2.25, 2.50.
    //   open 25, high 25, low 25, close 25;
    //   volume base: 25 * 3 = 75; volume quote: 625 * 3 = 1875.
    for _ in 0..10 {
        create_pair_prices(&mut suite, &mut accounts, &contracts).await?;
    }

    suite.app.indexer.wait_for_finish()?;

    let pair_prices = PairPriceQueryBuilder::new("dango".to_string(), "bridge/usdc".to_string())
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?
        .pair_prices;

    assert_that!(pair_prices.clone().len()).is_equal_to(10);

    let candles_1s = CandleQueryBuilder::new(
        CandleInterval::OneSecond,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    )
    .fetch_all(clickhouse_context.clickhouse_client())
    .await?
    .candles;

    // Note: this vector goes from the newest to the oldest candle.
    assert_that!(candles_1s).is_equal_to(vec![
        Candle {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
            time_start: MOCK_GENESIS_TIMESTAMP.to_utc_date_time() + TimeDelta::seconds(2),
            open: Udec128_24::new(25),
            high: Udec128_24::new(25),
            low: Udec128_24::new(25),
            close: Udec128_24::new(25),
            volume_base: Udec128_6::new(75),
            volume_quote: Udec128_6::new(1875),
            interval: CandleInterval::OneSecond,
            min_block_height: 8,
            max_block_height: 10,
        },
        Candle {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
            time_start: MOCK_GENESIS_TIMESTAMP.to_utc_date_time() + TimeDelta::seconds(1),
            open: Udec128_24::new(25),
            high: Udec128_24::new(25),
            low: Udec128_24::new(25),
            close: Udec128_24::new(25),
            volume_base: Udec128_6::new(100),
            volume_quote: Udec128_6::new(2500),
            interval: CandleInterval::OneSecond,
            min_block_height: 4,
            max_block_height: 7,
        },
        Candle {
            base_denom: "dango".to_string(),
            quote_denom: "bridge/usdc".to_string(),
            time_start: MOCK_GENESIS_TIMESTAMP.to_utc_date_time(),
            open: Udec128_24::from_str("27.5").unwrap(),
            high: Udec128_24::from_str("27.5").unwrap(),
            low: Udec128_24::new(25),
            close: Udec128_24::new(25),
            volume_base: Udec128_6::new(75),
            volume_quote: Udec128_6::from_str("1937.5").unwrap(),
            interval: CandleInterval::OneSecond,
            min_block_height: 1,
            max_block_height: 3,
        },
    ]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_changing_prices() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption {
            // Start at block 0 at 1 second, with a block time of 20 seconds.
            block_time: Duration::from_seconds(20),
            genesis_block: BlockInfo {
                height: 0,
                timestamp: Timestamp::from_seconds(1),
                hash: Hash256::ZERO,
            },
            ..Default::default()
        })
        .await;

    // Update dango's oracle price. It isn't used in this test but is required
    // for the contract to function.
    // Block time 21.
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(200),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Make an empty block at timestamps 41.
    suite.make_empty_block();

    // This function makes a block containing a single limit buy order and a
    // limit sell order of the same price and size. This produces a clearing
    // price that is to be indexed.
    let mut make_block_with_price =
        |suite: &mut TestSuite<_, _, _, _>, price, amount: Int<u128>| {
            let amount_in_quote = amount.checked_mul_dec_ceil(price).unwrap();
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates_market: vec![],
                        creates_limit: vec![
                            CreateLimitOrderRequest::Ask {
                                base_denom: dango::DENOM.clone(),
                                quote_denom: usdc::DENOM.clone(),
                                amount_base: NonZero::new_unchecked(amount),
                                price: NonZero::new_unchecked(price),
                            },
                            CreateLimitOrderRequest::Bid {
                                base_denom: dango::DENOM.clone(),
                                quote_denom: usdc::DENOM.clone(),
                                amount_quote: NonZero::new_unchecked(amount_in_quote),
                                price: NonZero::new_unchecked(price),
                            },
                        ],
                        cancels: None,
                    },
                    coins! {
                        dango::DENOM.clone() => amount,
                        usdc::DENOM.clone() => amount_in_quote,
                    },
                )
                .should_succeed();
        };

    let assert_candle = |candle: &Candle, open, close, low, high, vol_base, vol_quote, time| {
        assert_that!(candle.open).is_equal_to(Udec128_24::new(open));
        assert_that!(candle.close).is_equal_to(Udec128_24::new(close));
        assert_that!(candle.low).is_equal_to(Udec128_24::new(low));
        assert_that!(candle.high).is_equal_to(Udec128_24::new(high));
        assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(vol_base));
        assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(vol_quote));
        assert_that!(candle.time_start.naive_utc())
            .is_equal_to(DateTime::parse_from_rfc3339(time).unwrap().naive_utc());
    };

    let candle_query_builder = CandleQueryBuilder::new(
        CandleInterval::OneMinute,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    );

    // -------------------------------- block 1 --------------------------------

    // Block 1
    // Block time: 61 seconds
    // Price: 100_000
    // Volume in DANGO: 1
    // Volume in USDC: 1 * 100_000 = 100_000
    make_block_with_price(&mut suite, Udec128_24::new(100_000), Uint128::new(1));

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);
    assert_candle(
        &candle_1m.candles[0],
        100_000,
        100_000,
        100_000,
        100_000,
        1,
        100_000,
        "1970-01-01T00:01:00Z",
    );

    // -------------------------------- block 2 --------------------------------

    // Block 2
    // Block time: 81 seconds
    // Price: 99_999
    // Volume in DANGO: 1 + 1 (from previous block) = 2
    // Volume in USDC: 99_999 + 100_000 (from previous block) = 199_999
    make_block_with_price(&mut suite, Udec128_24::new(99_999), Uint128::new(1));

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);
    assert_candle(
        &candle_1m.candles[0],
        100_000,
        99_999,
        99_999,
        100_000,
        2,
        199_999,
        "1970-01-01T00:01:00Z",
    );

    // -------------------------------- block 3 --------------------------------

    // Block 3
    // Block time: 101 seconds
    // Price: 100_001
    // Volume in DANGO: 1 + 2 (from previous blocks) = 3
    // Volume in USDC: 100_001 + 199_999 (from previous blocks) = 300_000
    make_block_with_price(&mut suite, Udec128_24::new(100_001), Uint128::new(1));

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);
    assert_candle(
        &candle_1m.candles[0],
        100_000,
        100_001,
        99_999,
        100_001,
        3,
        300_000,
        "1970-01-01T00:01:00Z",
    );

    // -------------------------------- block 4 --------------------------------

    // Block 4
    // Block time: 121 seconds
    // Do nothing.
    suite.make_empty_block();

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    // Ensure there are two candles.
    // The most recent candle is the first; the oldest is the last.
    //
    // Since no trade happened in the new candle, its prices are all inherited
    // from the close price of the previous candle, and volume is zero.
    assert_that!(candle_1m.candles).has_length(2);
    assert_candle(
        &candle_1m.candles[0],
        100_001,
        100_001,
        100_001,
        100_001,
        0,
        0,
        "1970-01-01T00:02:00Z",
    );
    assert_candle(
        &candle_1m.candles[1],
        100_000,
        100_001,
        99_999,
        100_001,
        3,
        300_000,
        "1970-01-01T00:01:00Z",
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_pair_prices_with_small_amounts() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption {
            // Start at block 0 at 1 second, with a block time of 20 seconds.
            block_time: Duration::from_seconds(20),
            genesis_block: BlockInfo {
                height: 0,
                timestamp: Timestamp::from_seconds(1),
                hash: Hash256::ZERO,
            },
            ..Default::default()
        })
        .await;

    // Update dango's oracle price. It isn't used in this test but is required
    // for the contract to function.
    // Block time 21.
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(200),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Make an empty block at timestamps 41.
    suite.make_empty_block();

    // -------------------------------- block 1 --------------------------------

    // Block 1
    // Block time: 61 seconds
    // Place the following orders:
    // - limit, sell, price 100000 USDC per DANGO, size 2 DANGO
    // - limit, buy, price 100000, size 1 DANGO
    // - market, buy, size 1 DANGO
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest::Ask {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        amount_base: NonZero::new_unchecked(Uint128::new(20000000000000)),
                        price: NonZero::new_unchecked(
                            Udec128_24::from_str("0.000000003836916198").unwrap(),
                        ),
                    },
                    CreateLimitOrderRequest::Bid {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        amount_quote: NonZero::new_unchecked(Uint128::new(76739)),
                        price: NonZero::new_unchecked(
                            Udec128_24::from_str("0.000000003836916198").unwrap(),
                        ),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(20000000000000),
                usdc::DENOM.clone() => Uint128::new(76739),
            },
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let pair_price_query_builder =
        PairPriceQueryBuilder::new("dango".to_string(), "bridge/usdc".to_string()).with_limit(1);

    let pair_price = pair_price_query_builder
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .expect("Pair price should be found");

    assert_that!(pair_price.clearing_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);

    Ok(())
}

/// The auction over the orders in `orders_to_submit` should find maximum volume
/// in the range 25--30. Since no previous mid price exists, it takes the mid
/// point of the range, which is 27.5.
///
/// After the order has been filled, the remaining best bid is 20, best ask
/// is 25, so mid price is 22.5.
///
/// Summary:
/// - If this function is run a single time, the clearing price is 27.5.
/// - If it is run more than one times, any subsequent calls with have clearing
///   price of 25, because the previous auction's mid price (22.5) is smaller
///   than the lower bound of the range (25--30), so the lower bound (25) is
///   chosen.
async fn create_pair_prices(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> anyhow::Result<()> {
    let orders_to_submit: Vec<(Direction, u128, u128)> = vec![
        (Direction::Bid, 30, 25), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask, 5, 10),  //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - 50% filled
    ];

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128_24::new(price);
            let amount = Uint128::new(amount);

            let (funds, request) = create_limit_order_request(
                dango::DENOM.clone(),
                usdc::DENOM.clone(),
                direction,
                amount,
                price,
            );

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![request],
                    cancels: None,
                },
                funds,
            )?;

            signer.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    Ok(())
}
