use {
    assertor::*,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, ResultExt, Signer,
        StdResult, Timestamp, Udec128, Uint128, btree_map, setup_tracing_subscriber,
    },
    grug_app::Indexer,
    indexer_clickhouse::{
        Dec, Int,
        entities::{candle::Candle, pair_price::PairPrice},
    },
    tracing::Level,
};

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_mocked_clickhouse() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(false).await;

    // SetupBuilder::new()
    //     .with_mocked_clickhouse()
    //     .with_dango_indexer()
    //     .with_hyperlane()
    //     .build();

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
            let price = Udec128::new(price);
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
    assert_that!(pair_price.clearing_price).is_greater_than::<Dec<Udec128>>(Udec128::ZERO.into());

    Ok(())
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(true).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let pair_price = clickhouse_context
        .clickhouse_client()
        .query("SELECT * FROM pair_prices")
        .fetch_one::<PairPrice>()
        .await?;

    // Manual asserts so if clearing price changes, it doesn't break this test.
    assert_that!(pair_price.quote_denom).is_equal_to("bridge/usdc".to_string());
    assert_that!(pair_price.base_denom).is_equal_to("dango".to_string());
    assert_that!(pair_price.clearing_price).is_greater_than::<Dec<Udec128>>(Udec128::ZERO.into());
    assert_that!(pair_price.volume_base).is_equal_to::<Int<Uint128>>(Uint128::from(25).into());
    assert_that!(pair_price.volume_quote).is_equal_to::<Int<Uint128>>(Uint128::from(718).into());

    // Makes sure we get correct precision: 27.4 without specific number, since this can change.
    assert_that!(pair_price.clearing_price.to_string().len()).is_equal_to(4);

    let candle_1m: Candle = clickhouse_context
        .clickhouse_client()
        .query("SELECT *, '1m' as interval FROM pair_prices_1m")
        .fetch_one()
        .await?;

    let expected_candle = serde_json::json!({
        "quote_denom": "bridge/usdc",
        "base_denom": "dango",
        "close": 27400000000000000000_u128,
        "high":  27400000000000000000_u128,
        "interval": "1m",
        "low": 27400000000000000000_u128,
        "open": 27400000000000000000_u128,
        "time_start": serde_json::Number::from(31536000000000_u64),
        "volume_base": 25,
        "volume_quote": 718,
    });

    let candle_1m_serde =
        serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&candle_1m).unwrap())
            .unwrap();
    assert_that!(candle_1m_serde).is_equal_to(expected_candle);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles_with_real_clickhouse_and_one_minute_interval() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(true).await;

    for _ in 0..10 {
        create_pair_prices(&mut suite, &mut accounts, &contracts).await?;
    }

    suite.app.indexer.wait_for_finish()?;

    let pair_prices: Vec<PairPrice> = clickhouse_context
        .clickhouse_client()
        .query("SELECT * FROM pair_prices")
        .fetch_all()
        .await?;

    println!("pair_prices: {:#?}", pair_prices.len());

    println!(
        "pair_prices: {:#?}",
        pair_prices
            .into_iter()
            .map(|p| p.created_at)
            .collect::<Vec<_>>()
    );

    let candle_1m: Vec<Candle> = clickhouse_context
        .clickhouse_client()
        .query("SELECT *, '1m' as interval FROM pair_prices_1m")
        .fetch_all()
        .await?;

    println!("candle_1m: {:#?}", candle_1m.len());
    println!(
        "candle_1m: {:#?}",
        candle_1m
            .into_iter()
            .map(|c| c.time_start)
            .collect::<Vec<_>>()
    );

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
            let price = Udec128::new(price);
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
