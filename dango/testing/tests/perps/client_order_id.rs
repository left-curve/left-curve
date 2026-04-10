use {
    crate::register_oracle_prices,
    dango_testing::{TestAccount, TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, OrderId, UserState},
    },
    grug::{Addressable, Coins, QuerierExt, ResultExt, Uint128},
    std::collections::BTreeMap,
};

/// Set up both users with $10,000 margin each.
fn setup_margins(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &dango_genesis::Contracts,
) {
    for user in [&mut accounts.user1, &mut accounts.user2] {
        suite
            .execute(
                user,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
                Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
            )
            .should_succeed();
    }
}

/// Place a post-only ask (sell) order with optional client_order_id.
fn place_ask(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    account: &mut TestAccount,
    contracts: &dango_genesis::Contracts,
    size: i128,
    price: i128,
    client_order_id: Option<&str>,
) {
    suite
        .execute(
            account,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair_id(),
                size: Quantity::new_int(-size.abs()),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(price),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: client_order_id.map(String::from),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();
}

/// Place a post-only bid (buy) order with optional client_order_id.
fn place_bid(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    account: &mut TestAccount,
    contracts: &dango_genesis::Contracts,
    size: i128,
    price: i128,
    client_order_id: Option<&str>,
) {
    suite
        .execute(
            account,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair_id(),
                size: Quantity::new_int(size.abs()),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(price),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: client_order_id.map(String::from),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();
}

/// Query all orders for a user, keyed by system order ID.
fn query_orders(
    suite: &dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    contracts: &dango_genesis::Contracts,
    user: grug::Addr,
) -> BTreeMap<OrderId, perps::QueryOrdersByUserResponseItem> {
    suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest { user })
        .should_succeed()
}

// =============================================================================
// Tests
// =============================================================================

/// Place a resting order with a client_order_id, cancel it by client_order_id,
/// and verify the order is removed and margin is released.
#[test]
fn cancel_order_by_client_order_id() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    // Place a resting ask with client_order_id.
    place_ask(
        &mut suite,
        &mut accounts.user1,
        &contracts,
        5,
        2_000,
        Some("my-order-1"),
    );

    // Verify order is on the book with the client_order_id.
    let orders = query_orders(&suite, &contracts, accounts.user1.address());
    assert_eq!(orders.len(), 1);
    let (_, item) = orders.iter().next().unwrap();
    assert_eq!(item.client_order_id.as_deref(), Some("my-order-1"));

    // Cancel by client_order_id.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::ByClientOrderId("my-order-1".into()),
            )),
            Coins::new(),
        )
        .should_succeed();

    // Verify order is gone.
    let orders = query_orders(&suite, &contracts, accounts.user1.address());
    assert!(orders.is_empty(), "order should be removed after cancel");

    // Verify margin released.
    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    assert_eq!(state.open_order_count, 0);
    assert_eq!(state.reserved_margin, UsdValue::ZERO);
}

/// Cancel by a client_order_id that doesn't exist — should fail.
#[test]
fn cancel_nonexistent_client_order_id_fails() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::ByClientOrderId("nonexistent".into()),
            )),
            Coins::new(),
        )
        .should_fail_with_error("no active order with client_order_id");
}

/// Submit two orders with the same client_order_id — second should be rejected.
#[test]
fn duplicate_client_order_id_rejected() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    place_ask(
        &mut suite,
        &mut accounts.user1,
        &contracts,
        5,
        2_000,
        Some("dup-id"),
    );

    // Second order with same client_order_id should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair_id(),
                size: Quantity::new_int(3),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_900),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: Some("dup-id".into()),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("duplicate client_order_id");
}

/// After cancelling an order, its client_order_id can be reused.
#[test]
fn client_order_id_reusable_after_cancel() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    // Place and cancel.
    place_ask(
        &mut suite,
        &mut accounts.user1,
        &contracts,
        5,
        2_000,
        Some("reuse-me"),
    );

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::ByClientOrderId("reuse-me".into()),
            )),
            Coins::new(),
        )
        .should_succeed();

    // Reuse the same client_order_id — should succeed.
    place_bid(
        &mut suite,
        &mut accounts.user1,
        &contracts,
        3,
        1_900,
        Some("reuse-me"),
    );

    let orders = query_orders(&suite, &contracts, accounts.user1.address());
    assert_eq!(orders.len(), 1);
    let (_, item) = orders.iter().next().unwrap();
    assert_eq!(item.client_order_id.as_deref(), Some("reuse-me"));
}

/// Different users can use the same client_order_id simultaneously.
#[test]
fn different_users_same_client_order_id() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    // Both users place orders with the same client_order_id.
    place_ask(
        &mut suite,
        &mut accounts.user1,
        &contracts,
        5,
        2_000,
        Some("shared-id"),
    );
    place_ask(
        &mut suite,
        &mut accounts.user2,
        &contracts,
        3,
        2_100,
        Some("shared-id"),
    );

    // Both should exist.
    let orders1 = query_orders(&suite, &contracts, accounts.user1.address());
    let orders2 = query_orders(&suite, &contracts, accounts.user2.address());
    assert_eq!(orders1.len(), 1);
    assert_eq!(orders2.len(), 1);

    // user1 cancels by client_order_id — only their order is removed.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::ByClientOrderId("shared-id".into()),
            )),
            Coins::new(),
        )
        .should_succeed();

    let orders1 = query_orders(&suite, &contracts, accounts.user1.address());
    let orders2 = query_orders(&suite, &contracts, accounts.user2.address());
    assert!(orders1.is_empty(), "user1 order should be cancelled");
    assert_eq!(orders2.len(), 1, "user2 order should still exist");
}

/// Orders without a client_order_id still work — the field is optional.
#[test]
fn order_without_client_order_id() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    setup_margins(&mut suite, &mut accounts, &contracts);

    place_ask(&mut suite, &mut accounts.user1, &contracts, 5, 2_000, None);

    let orders = query_orders(&suite, &contracts, accounts.user1.address());
    assert_eq!(orders.len(), 1);
    let (_, item) = orders.iter().next().unwrap();
    assert_eq!(item.client_order_id, None);
}
