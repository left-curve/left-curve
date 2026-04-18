use {
    crate::register_oracle_prices,
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Quantity, UsdPrice,
        constants::usdc,
        perps::{self, ClientOrderId},
    },
    grug::{Addressable, Coins, QuerierExt, ResultExt, Uint64, Uint128},
    std::collections::BTreeMap,
};

/// End-to-end: submit a GTC limit order carrying a `client_order_id`,
/// cancel it via `CancelOrderRequest::OneByClientOrderId`, and prove the
/// same `client_order_id` is reusable after the cancel.
///
/// Covers the full execute-message round trip the algo-trader use case
/// depends on.
#[test]
fn submit_cancel_resubmit_by_client_order_id() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();
    let cid: ClientOrderId = Uint64::new(42);

    // Deposit margin so the trader can place a resting order.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Submit a resting GTC limit bid carrying `cid`. The price ($1,500) is
    // well below the oracle ($2,000) so the order rests rather than crosses.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_500),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                    client_order_id: Some(cid),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Sanity: exactly one resting order, carrying `cid`.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(orders.len(), 1);

    // Cancel it by the client order id — the trader doesn't need to know
    // the system-assigned `OrderId`.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::OneByClientOrderId(cid),
            )),
            Coins::new(),
        )
        .should_succeed();

    // The order is gone.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert!(orders.is_empty(), "order should be removed after cancel");

    // The same `cid` is reusable now that the prior order is no longer
    // active. (If the alias hadn't been cleared, this would hit
    // `StdError::duplicate_data` from the `client_order_id` UniqueIndex.)
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_400),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                    client_order_id: Some(cid),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(
        orders.len(),
        1,
        "the re-submitted order should be on the book"
    );
}

/// Cancelling a `client_order_id` that the sender never used (or has
/// already cancelled / had filled) bails with a clear error message.
#[test]
fn cancel_by_unknown_client_order_id_fails() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::OneByClientOrderId(Uint64::new(99)),
            )),
            Coins::new(),
        )
        .should_fail_with_error("order not found");
}
