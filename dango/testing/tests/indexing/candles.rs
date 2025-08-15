use {
    assertor::*,
    chrono::DateTime,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, CreateMarketOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        BlockInfo, Bounded, Coins, Duration, Hash256, Message, MultiplyFraction, NonEmpty, NonZero,
        Number, NumberConst, ResultExt, Signer, StdResult, Timestamp, Udec128, Udec128_6,
        Udec128_24, Uint128, btree_map, coins,
    },
    grug_app::Indexer,
    indexer_clickhouse::entities::{
        CandleInterval, candle_query::CandleQueryBuilder, pair_price::PairPrice,
        pair_price_query::PairPriceQueryBuilder,
    },
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

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction,
                        amount: NonZero::new_unchecked(amount),
                        price: NonZero::new_unchecked(price),
                    }],
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
    assert_that!(pair_price.close_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);

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
    assert_that!(pair_price.close_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);
    assert_that!(pair_price.volume_base)
        .is_equal_to::<Udec128_6>(Udec128_6::from_str("25.0").unwrap());
    assert_that!(pair_price.volume_quote)
        .is_equal_to::<Udec128_6>(Udec128_6::from_str("687.5").unwrap());

    // Makes sure we get correct precision: 27.4 without specific number, since this can change.
    assert_that!(pair_price.close_price.to_string().len()).is_equal_to(4);

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

    let candle_query_builder = CandleQueryBuilder::new(
        CandleInterval::OneMinute,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    );

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);

    assert_that!(candle_1m.has_next_page).is_false();
    assert_that!(candle_1m.has_previous_page).is_false();

    assert_that!(candle_1m.candles[0].time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1971-01-01T00:00:00Z")
            .unwrap()
            .naive_utc(),
    );

    assert_that!(candle_1m.candles[0].open).is_equal_to(pair_prices[0].clone().close_price);
    assert_that!(candle_1m.candles[0].high).is_equal_to(pair_prices[0].clone().close_price);
    assert_that!(candle_1m.candles[0].low).is_equal_to(pair_prices[0].clone().close_price);
    assert_that!(candle_1m.candles[0].close).is_equal_to(pair_prices[0].clone().close_price);
    assert_that!(candle_1m.candles[0].volume_base).is_equal_to(
        pair_prices[0]
            .clone()
            .volume_base
            .checked_mul(Udec128_6::from_str("10.0").unwrap())
            .unwrap(),
    );
    assert_that!(candle_1m.candles[0].volume_quote).is_equal_to(
        pair_prices[0]
            .clone()
            .volume_quote
            .checked_mul(Udec128_6::from_str("10.0").unwrap())
            .unwrap(),
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse_and_one_second_interval() -> anyhow::Result<()> {
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

    let candle_query_builder = CandleQueryBuilder::new(
        CandleInterval::OneSecond,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    );

    let candle_1s = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1s.candles).has_length(6);
    assert_that!(
        candle_1s
            .candles
            .iter()
            .map(|c| c.time_start.naive_utc().to_string())
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![
        "1971-01-01 00:00:05".to_string(),
        "1971-01-01 00:00:04".to_string(),
        "1971-01-01 00:00:03".to_string(),
        "1971-01-01 00:00:02".to_string(),
        "1971-01-01 00:00:01".to_string(),
        "1971-01-01 00:00:00".to_string(),
    ]);

    assert_that!(
        candle_1s
            .candles
            .iter()
            .map(|c| &c.volume_quote)
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![
        &Udec128_6::from_str("625").unwrap(),
        &Udec128_6::from_str("1250").unwrap(),
        &Udec128_6::from_str("1250").unwrap(),
        &Udec128_6::from_str("1250").unwrap(),
        &Udec128_6::from_str("1250").unwrap(),
        &Udec128_6::from_str("687.5").unwrap(),
    ]);

    assert_that!(
        candle_1s
            .candles
            .iter()
            .map(|c| &c.volume_base)
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![
        &Udec128_6::from_str("25").unwrap(),
        &Udec128_6::from_str("50").unwrap(),
        &Udec128_6::from_str("50").unwrap(),
        &Udec128_6::from_str("50").unwrap(),
        &Udec128_6::from_str("50").unwrap(),
        &Udec128_6::from_str("25").unwrap(),
    ]);

    assert_that!(candle_1s.candles[5].open)
        .is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1s.candles[5].high)
        .is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1s.candles[5].low)
        .is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
    assert_that!(candle_1s.candles[5].close)
        .is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());

    for candle in candle_1s.candles.into_iter().rev().skip(1) {
        assert_that!(candle.open).is_equal_to::<Udec128_24>(Udec128_24::from_str("25").unwrap());
        assert_that!(candle.high).is_equal_to::<Udec128_24>(Udec128_24::from_str("25").unwrap());
        assert_that!(candle.low).is_equal_to::<Udec128_24>(Udec128_24::from_str("25").unwrap());
        assert_that!(candle.close).is_equal_to::<Udec128_24>(Udec128_24::from_str("25").unwrap());
    }

    Ok(())
}

/// A comprehensive test that covers all cases:
/// - limit bid matched against limit ask
/// - limit bid matched against market ask
/// - limit ask matched against limit bid
/// - limit ask matched against market bid
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_both_market_and_limit_orders_one_minute_interval() -> anyhow::Result<()>
{
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

    // -------------------------------- First trades (two blocks) --------------------------------

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
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(3)),
                        price: NonZero::new_unchecked(Udec128_24::new(100_000)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(1)),
                        price: NonZero::new_unchecked(Udec128_24::new(100_000)),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(3),
                usdc::DENOM.clone() => Uint128::new(100_000),
            },
        )
        .should_succeed();

    // Block time: 81 seconds
    // Create a market order to fill the limit order.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(1)),
                    max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                }],
                creates_limit: vec![],
                cancels: None,
            },
            coins! {
                usdc::DENOM.clone() => Uint128::new(100_000),
            },
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let candle_query_builder = CandleQueryBuilder::new(
        CandleInterval::OneMinute,
        "dango".to_string(),
        "bridge/usdc".to_string(),
    );

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert!(
        candle_1m.candles.len() == 1,
        "Expected one candle after first block, received: {candle_1m:#?}"
    );

    let candle = &candle_1m.candles[0];

    // time 60-120, open 100_000, high 100_000, low 100_000, close 100_000, volume 200_000 USD

    assert_that!(candle.open).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(2));
    assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(200000));
    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1970-01-01T00:01:00Z")
            .unwrap()
            .naive_utc(),
    );

    // -------------------------------- Third block --------------------------------

    // Block time: 101 seconds
    // Place the following orders:
    // - limit, sell, price 99999, size 2
    // - limit, buy, price 100000, size 1
    // - market, buy, size 1 (becomes limit at 100_000 since one limit ask at 100_000 remains in the order book)
    // Clearing price is 99_999, total volume: 99999 + 99999 = 199998 USD.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(1)), // price 99_999 * size 1
                    max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                }],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(2)),
                        price: NonZero::new_unchecked(Udec128_24::new(99_999)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(1)),
                        price: NonZero::new_unchecked(Udec128_24::new(99_999)),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(2),
                usdc::DENOM.clone() => Uint128::new(200_000),
            },
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    // time 60-120, open 100_000, high 100_000, low 99_999, close 99_999.5, volume 399_998.5 (200_000 + 199_998.5)

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);

    let candle = &candle_1m.candles[0];

    assert_that!(candle.open).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(4));
    assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(399_998));
    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1970-01-01T00:01:00Z")
            .unwrap()
            .naive_utc(),
    );

    // -------------------------------- Fourth block (First block in second candle) --------------------------------

    // Block 3
    // Block time: 121 seconds
    // Place the following orders:
    // - limit, sell, price 100000, size 1
    // - limit, buy, price 100001, size 2
    // Total volume: 100_000 + 100_000 = 200_000 USD. (One limit ask at 100_000 remains in the order book from the previous block)
    // After this the book is empty.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(1)),
                        price: NonZero::new_unchecked(Udec128_24::new(100_000)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(2)),
                        price: NonZero::new_unchecked(Udec128_24::new(100_001)),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(1),
                usdc::DENOM.clone() => Uint128::new(200_002), // price 100_001 * size 2
            },
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(2);

    let candle = &candle_1m.candles[0];

    // time 120-180
    assert_that!(candle.open).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(100_000));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(100_000));
    assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(2));
    assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(200000));
    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1970-01-01T00:02:00Z")
            .unwrap()
            .naive_utc(),
    );

    // -------------------------------- block 4 --------------------------------

    // Block time: 141 seconds
    // Place the following orders:
    // - limit, sell, price 99999, size 1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(1)),
                    price: NonZero::new_unchecked(Udec128_24::new(99_998)),
                }],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(1),
            },
        )
        .should_succeed();

    // Block time: 161 seconds
    // Place the following orders:
    // - market, buy, size 1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(1)),
                    max_slippage: Bounded::new_unchecked(Udec128::ZERO),
                }],
                creates_limit: vec![],
                cancels: None,
            },
            coins! {
                usdc::DENOM.clone() => Uint128::new(99_998),
            },
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let candle_1m = candle_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    // ensure there are two candles
    assert_that!(candle_1m.candles).has_length(2);

    // Most recent is first, oldest is last.
    let candle = &candle_1m.candles[1];

    // Oldest candle
    // time 60-120, open 100_000, high 100_001, low 99_999, close 100_000.5, volume 600_000
    assert_that!(candle.open).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(100000));
    assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(4));
    assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(399_998));
    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1970-01-01T00:01:00Z")
            .unwrap()
            .naive_utc(),
    );

    let candle = &candle_1m.candles[0];

    // Most recent candle
    // time 120-180, open 100_000.5, high 100_000.5, low 100_000.5, close 100_000.5, volume 0
    assert_that!(candle.open).is_equal_to(Udec128_24::new(99_999));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(99_998));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(99_998));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(100_000));
    assert_that!(candle.volume_base).is_equal_to(Udec128_6::new(3));
    assert_that!(candle.volume_quote).is_equal_to(Udec128_6::new(299_998));
    assert_that!(candle.time_start.naive_utc()).is_equal_to(
        DateTime::parse_from_rfc3339("1970-01-01T00:02:00Z")
            .unwrap()
            .naive_utc(),
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
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(20000000000000)),
                        price: NonZero::new_unchecked(
                            Udec128_24::from_str("0.000000003836916198").unwrap(),
                        ),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(20000000000000)),
                        price: NonZero::new_unchecked(
                            Udec128_24::from_str("0.000000003836916198").unwrap(),
                        ),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => Uint128::new(20000000000000),
                usdc::DENOM.clone() => Uint128::new(200_000),
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

    assert_that!(pair_price.close_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);

    Ok(())
}

async fn create_pair_prices(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> anyhow::Result<()> {
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

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction,
                        amount: NonZero::new_unchecked(amount),
                        price: NonZero::new_unchecked(price),
                    }],
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
