use {
    assertor::*,
    dango_genesis::Contracts,
    dango_indexer_clickhouse::entities::trade_query::TradeQueryBuilder,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateOrderRequest, Direction},
    },
    grug::{
        Coin, Coins, Message, MultiplyFraction, NonEmpty, NonZero, ResultExt, Signer, StdResult,
        Udec128_24, Uint128,
    },
    grug_app::Indexer,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_trades() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let trade_query_builder = TradeQueryBuilder::default();

    let trades = trade_query_builder
        .fetch_all(clickhouse_context.clickhouse_client())
        .await?;

    assert_that!(trades.trades).has_length(4);

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

            let fund = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coin::new(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coin::new(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        direction,
                        NonZero::new_unchecked(price),
                        NonZero::new_unchecked(fund.amount),
                    )],
                    cancels: None,
                },
                Coins::from(fund),
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
