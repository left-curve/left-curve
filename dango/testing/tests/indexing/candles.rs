use {
    assertor::*,
    dango_testing::setup_test_with_indexer,
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
    indexer_clickhouse::entities::pair_price::PairPrice,
    tracing::Level,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);

    let (mut suite, mut accounts, _, contracts, _, _, _, clickhouse_context) =
        setup_test_with_indexer().await;

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

    tracing::info!("{:#?}", clickhouse_inserts);

    Ok(())
}
