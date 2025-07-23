use {
    assert_json_diff::assert_json_include,
    assertor::*,
    chrono::DateTime,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        Coins, Message, MultiplyFraction, NonEmpty, NonZero, Number, NumberConst, ResultExt,
        Signer, StdResult, Timestamp, Udec128, Udec128_6, Udec128_24, Uint128, btree_map,
        setup_tracing_subscriber,
    },
    grug_app::Indexer,
    indexer_clickhouse::entities::{
        CandleInterval, candle_query::CandleQueryBuilder, pair_price::PairPrice,
        pair_price_query::PairPriceQueryBuilder,
    },
    std::str::FromStr,
    tracing::Level,
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
                        price,
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
    assert_that!(pair_price.clearing_price).is_greater_than::<Udec128_24>(Udec128_24::ZERO);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let pair_price_query_builder =
        PairPriceQueryBuilder::new("dango".to_string(), "bridge/usdc".to_string());

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
    );

    let candle_1m = candle_query_builder
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .unwrap();

    // `PairPrice` is serialized as u128 for clickhouse
    let expected_candle = serde_json::json!({
        "quote_denom": "bridge/usdc",
        "base_denom": "dango",
        "close": 27500000000000000000000000_u128,
        "high":  27500000000000000000000000_u128,
        "interval": "1m",
        "low": 27500000000000000000000000_u128,
        "open": 27500000000000000000000000_u128,
        "time_start": serde_json::Number::from(31536000000000_u64),
        "volume_base": 25000000,
        "volume_quote": 687500000,
    });

    let candle_1m_serde =
        serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&candle_1m).unwrap())
            .unwrap();

    assert_json_include!(actual: candle_1m_serde, expected: expected_candle);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse_and_one_minute_interval() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);
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

    assert_that!(pair_prices.clone()).has_length(18); // 10 paire_prices when not filling

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

    assert_that!(candle_1m.candles[0].open).is_equal_to(pair_prices[0].clone().clearing_price);
    assert_that!(candle_1m.candles[0].high).is_equal_to(pair_prices[0].clone().clearing_price);
    assert_that!(candle_1m.candles[0].low).is_equal_to(pair_prices[0].clone().clearing_price);
    assert_that!(candle_1m.candles[0].close).is_equal_to(pair_prices[0].clone().clearing_price);
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
    setup_tracing_subscriber(Level::INFO);
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

    assert_that!(pair_prices.clone()).has_length(18); // 10 paire_prices when not filling with empty prices

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
        &Udec128_6::from_str("687.5").unwrap(),
        &Udec128_6::from_str("1375").unwrap(),
        &Udec128_6::from_str("1375").unwrap(),
        &Udec128_6::from_str("1375").unwrap(),
        &Udec128_6::from_str("1375").unwrap(),
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

    for candle in candle_1s.candles.into_iter() {
        assert_that!(candle.open).is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
        assert_that!(candle.high).is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
        assert_that!(candle.low).is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
        assert_that!(candle.close).is_equal_to::<Udec128_24>(Udec128_24::from_str("27.5").unwrap());
    }

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
                        price,
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
