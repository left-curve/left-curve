use {
    assertor::*,
    dango_genesis::Contracts,
    dango_indexer_clickhouse::entities::{
        CandleInterval, perps_candle_query::PerpsCandleQueryBuilder,
        perps_pair_price::PerpsPairPrice,
    },
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        constants::usdc,
        oracle::{self, PriceSource},
        perps,
    },
    grug::{
        Coins, Denom, NumberConst, ResultExt, Timestamp, Udec128, Udec128_6, Udec128_24, Uint128,
        btree_map,
    },
    grug_app::Indexer,
};

fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Register fixed oracle prices for the perps pair and settlement currency.
fn register_oracle_prices(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
                pair_id() => PriceSource::Fixed {
                    humanized_price: Udec128::new(eth_price),
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
}

/// Deposit USDC margin into the perps contract for a user.
fn deposit_margin(
    suite: &mut TestSuiteWithIndexer,
    account: &mut dango_testing::TestAccount,
    contracts: &Contracts,
    amount_usd: u128,
) {
    // USDC has 6 decimals: $X = X * 10^6
    let amount = Uint128::new(amount_usd * 1_000_000);
    suite
        .execute(
            account,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), amount).unwrap(),
        )
        .should_succeed();
}

/// Place a limit order and let it fill by submitting a market counter-order.
///
/// Returns after the block is mined (so OrderFilled events are emitted).
///
/// `price` is the USD price per unit (e.g. 2000 for ETH @ $2000).
/// `size` is the number of units (e.g. 10 for 10 ETH).
async fn create_perps_fill(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    price: u128,
    size: u128,
) {
    let pair = pair_id();

    // Maker (user2) places a limit ask
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-(size as i128)), // negative = sell/ask
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(price as i128),
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Taker (user1) places a market buy that crosses the ask
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(size as i128), // positive = buy/long
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn index_perps_candles_basic() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // Both users deposit sufficient margin.
    deposit_margin(&mut suite, &mut accounts.user1, &contracts, 100_000);
    deposit_margin(&mut suite, &mut accounts.user2, &contracts, 100_000);

    // Create a fill: 5 ETH @ $2000
    create_perps_fill(&mut suite, &mut accounts, &contracts, 2_000, 5).await;

    suite.app.indexer.wait_for_finish().await?;

    // Query perps pair prices from ClickHouse
    let pair_prices =
        PerpsPairPrice::latest_prices(clickhouse_context.clickhouse_client(), 10).await?;

    // Should have at least one pair price entry
    assert_that!(pair_prices.len()).is_at_least(1);

    // Find the pair price for our pair
    let pp = pair_prices
        .iter()
        .find(|p| p.pair_id == pair_id().to_string())
        .expect("Should find pair price for perp/ethusd");

    // Fill price should be $2000 → scaled to Udec128_24
    let expected_price = Udec128_24::new(2_000);
    assert_that!(pp.close).is_equal_to(expected_price);
    assert_that!(pp.high).is_equal_to(expected_price);
    assert_that!(pp.low).is_equal_to(expected_price);

    // Volume = sum(abs(fill_size)) across all OrderFilled events.
    // Each trade emits 2 OrderFilled (maker + taker), so volume = 5 + 5 = 10.
    assert_that!(pp.volume).is_equal_to(Udec128_6::new(10));

    // volume_usd = sum(abs(fill_size) * fill_price) = 5*2000 + 5*2000 = 20000
    assert_that!(pp.volume_usd).is_equal_to(Udec128_6::new(20_000));

    // Query candle for 1-minute interval
    let candle_1m = PerpsCandleQueryBuilder::new(CandleInterval::OneMinute, pair_id().to_string())
        .with_limit(1)
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .expect("Should have a 1-minute candle");

    assert_that!(candle_1m.pair_id).is_equal_to(pair_id().to_string());
    assert_that!(candle_1m.open).is_equal_to(expected_price);
    assert_that!(candle_1m.high).is_equal_to(expected_price);
    assert_that!(candle_1m.low).is_equal_to(expected_price);
    assert_that!(candle_1m.close).is_equal_to(expected_price);
    assert_that!(candle_1m.volume).is_equal_to(Udec128_6::new(10));
    assert_that!(candle_1m.volume_usd).is_equal_to(Udec128_6::new(20_000));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn index_perps_candles_multiple_fills_same_block() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    deposit_margin(&mut suite, &mut accounts.user1, &contracts, 100_000);
    deposit_margin(&mut suite, &mut accounts.user2, &contracts, 100_000);

    let pair = pair_id();

    // Place two limit asks at different prices, then a large market buy that
    // fills both in the same block (same cron execution).
    // Ask 1: 3 ETH @ $2000
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-3),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Ask 2: 2 ETH @ $2100
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-2),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_100),
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Market buy 5 ETH — should cross both asks producing 2 OrderFilled events
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    // Query pair price
    let pair_prices =
        PerpsPairPrice::latest_prices(clickhouse_context.clickhouse_client(), 10).await?;

    let pp = pair_prices
        .iter()
        .find(|p| p.pair_id == pair.to_string())
        .expect("Should find pair price");

    // With two fills at different prices in the same block:
    // high should be max(2000, 2100) = 2100
    // low should be min(2000, 2100) = 2000
    // close should be the last fill price
    assert_that!(pp.high).is_equal_to(Udec128_24::new(2_100));
    assert_that!(pp.low).is_equal_to(Udec128_24::new(2_000));

    // Total volume = (3+3) + (2+2) = 10 (each fill emits 2 OrderFilled events)
    assert_that!(pp.volume).is_equal_to(Udec128_6::new(10));

    // volume_usd = (3+3)*2000 + (2+2)*2100 = 12000 + 8400 = 20400
    assert_that!(pp.volume_usd).is_equal_to(Udec128_6::new(20_400));

    // Candle should aggregate correctly
    let candle_1m = PerpsCandleQueryBuilder::new(CandleInterval::OneMinute, pair.to_string())
        .with_limit(1)
        .fetch_one(clickhouse_context.clickhouse_client())
        .await?
        .expect("Should have a 1-minute candle");

    assert_that!(candle_1m.high).is_equal_to(Udec128_24::new(2_100));
    assert_that!(candle_1m.low).is_equal_to(Udec128_24::new(2_000));
    assert_that!(candle_1m.volume).is_equal_to(Udec128_6::new(10));
    assert_that!(candle_1m.volume_usd).is_equal_to(Udec128_6::new(20_400));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn index_perps_candles_changing_prices() -> anyhow::Result<()> {
    // Default block time is 250ms, so all fills land in the same second/minute.
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    deposit_margin(&mut suite, &mut accounts.user1, &contracts, 50_000);
    deposit_margin(&mut suite, &mut accounts.user2, &contracts, 50_000);

    let pair = pair_id();

    // Helper: place a limit ask then market buy to get a fill at a specific price
    let fill_at_price =
        |suite: &mut TestSuiteWithIndexer, accounts: &mut TestAccounts, price: i128, size: i128| {
            suite
                .execute(
                    &mut accounts.user2,
                    contracts.perps,
                    &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                        pair_id: pair.clone(),
                        size: Quantity::new_int(-size),
                        kind: perps::OrderKind::Limit {
                            limit_price: UsdPrice::new_int(price),
                            post_only: true,
                        },
                        reduce_only: false,
                    }),
                    Coins::new(),
                )
                .should_succeed();

            suite
                .execute(
                    &mut accounts.user1,
                    contracts.perps,
                    &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                        pair_id: pair.clone(),
                        size: Quantity::new_int(size),
                        kind: perps::OrderKind::Market {
                            max_slippage: Dimensionless::ONE,
                        },
                        reduce_only: false,
                    }),
                    Coins::new(),
                )
                .should_succeed();
        };

    // Fill 1: 1 ETH @ $2000
    fill_at_price(&mut suite, &mut accounts, 2_000, 1);

    // Fill 2: 1 ETH @ $1999 (price drops)
    fill_at_price(&mut suite, &mut accounts, 1_999, 1);

    // Fill 3: 1 ETH @ $2001 (price rises)
    fill_at_price(&mut suite, &mut accounts, 2_001, 1);

    suite.app.indexer.wait_for_finish().await?;

    // All fills should be in the same 1-minute candle (250ms block time)
    let candle_1m = PerpsCandleQueryBuilder::new(CandleInterval::OneMinute, pair.to_string())
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(candle_1m.candles).has_length(1);
    let candle = &candle_1m.candles[0];

    // OHLC: open from first fill, close from last fill, high/low from extremes
    assert_that!(candle.open).is_equal_to(Udec128_24::new(2_000));
    assert_that!(candle.close).is_equal_to(Udec128_24::new(2_001));
    assert_that!(candle.high).is_equal_to(Udec128_24::new(2_001));
    assert_that!(candle.low).is_equal_to(Udec128_24::new(1_999));

    // Volume: 3 fills * 2 events each * 1 ETH = 6
    assert_that!(candle.volume).is_equal_to(Udec128_6::new(6));

    Ok(())
}
