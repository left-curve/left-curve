use {
    crate::{default_pair_param, default_param, register_oracle_prices},
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        constants::usdc,
        perps::{self, CancelOrderRequest, Param, SubmitOrCancelOrderRequest, SubmitOrderRequest},
    },
    grug::{Addressable, Coins, NonEmpty, QuerierExt, ResultExt, Uint64, Uint128, btree_map},
    std::collections::BTreeMap,
};

fn limit_bid(price: i128, size: i128, cid: Option<u64>) -> SubmitOrderRequest {
    SubmitOrderRequest {
        pair_id: pair_id(),
        size: Quantity::new_int(size),
        kind: perps::OrderKind::Limit {
            limit_price: UsdPrice::new_int(price),
            time_in_force: perps::TimeInForce::PostOnly,
            client_order_id: cid.map(Uint64::new),
        },
        reduce_only: false,
        tp: None,
        sl: None,
    }
}

fn limit_ask(price: i128, size: i128, cid: Option<u64>) -> SubmitOrderRequest {
    SubmitOrderRequest {
        pair_id: pair_id(),
        size: Quantity::new_int(-size),
        kind: perps::OrderKind::Limit {
            limit_price: UsdPrice::new_int(price),
            time_in_force: perps::TimeInForce::PostOnly,
            client_order_id: cid.map(Uint64::new),
        },
        reduce_only: false,
        tp: None,
        sl: None,
    }
}

/// Happy path: batch of [PostOnly bid, PostOnly ask, Cancel(One(bid))].
/// Assert exactly one resting order remains (the ask).
#[test]
fn batch_submit_then_cancel() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Submit bid first to learn its OrderId, then run the batch.
    // Easier: use client_order_id on the bid to reference it in the cancel.
    let bid_cid = 1u64;
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, Some(bid_cid))),
                    SubmitOrCancelOrderRequest::Submit(limit_ask(2_100, 1, None)),
                    SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::OneByClientOrderId(
                        Uint64::new(bid_cid),
                    )),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_succeed();

    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(orders.len(), 1, "only the ask should remain on the book");
    let (_, ask) = orders.iter().next().unwrap();
    assert_eq!(ask.size, Quantity::new_int(-1));
}

/// Atomic replacement: user rests 3 orders, then issues a batch that
/// cancels all and re-submits 3 new ones. Assert the new orders are on
/// the book and the old ones are gone.
#[test]
fn batch_atomic_replace() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Rest 3 bids at descending prices.
    for price in [1_900, 1_800, 1_700] {
        suite
            .execute(
                &mut accounts.user1,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(limit_bid(price, 1, None))),
                Coins::new(),
            )
            .should_succeed();
    }

    let old_orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(old_orders.len(), 3);
    let old_ids: Vec<perps::OrderId> = old_orders.keys().copied().collect();

    // Batch: cancel all + 3 new submits at different prices.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::All),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_850, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_750, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_650, 1, None)),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_succeed();

    let new_orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(new_orders.len(), 3, "book should hold the 3 new bids");
    for id in old_ids {
        assert!(
            !new_orders.contains_key(&id),
            "old order id {id} should have been canceled"
        );
    }
}

/// Within one batch, a `Cancel(OneByClientOrderId(42))` releases the
/// client id so a subsequent `Submit` carrying the same client id
/// succeeds — the two actions see each other's writes via grug's
/// in-call `Buffer`.
#[test]
fn batch_reuse_client_order_id() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let cid = 42u64;

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Resting bid with client_order_id = 42.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(limit_bid(
                1_900,
                1,
                Some(cid),
            ))),
            Coins::new(),
        )
        .should_succeed();

    // Batch: cancel by cid, then submit a new order carrying the same cid.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::OneByClientOrderId(
                        Uint64::new(cid),
                    )),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_800, 1, Some(cid))),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_succeed();

    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(orders.len(), 1);
    let (_, order) = orders.iter().next().unwrap();
    assert_eq!(order.limit_price, UsdPrice::new_int(1_800));
}

/// A duplicate `client_order_id` within a batch fails the second submit
/// and rolls back the first. Snapshot `UserState` and `orders` before
/// the batch; assert they're byte-identical after the failure.
#[test]
fn batch_atomicity_on_submit_failure() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    let state_before: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let orders_before: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    // Two submits with the same client_order_id. The first is accepted by the
    // matching engine, the second collides on the unique-cid index and fails.
    let cid = 7u64;
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, Some(cid))),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_800, 1, Some(cid))),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_fail();

    let state_after: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let orders_after: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert_eq!(state_before, state_after, "UserState must be unchanged");
    assert_eq!(orders_before, orders_after, "orders must be unchanged");
}

/// A bogus `Cancel(One(...))` mid-batch rolls back an earlier successful
/// submit. Same snapshot-and-compare assertion as the submit-failure
/// case above.
#[test]
fn batch_atomicity_on_cancel_failure() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    let state_before: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let orders_before: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, None)),
                    SubmitOrCancelOrderRequest::Cancel(CancelOrderRequest::One(Uint64::new(
                        99_999,
                    ))),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_fail_with_error("order not found");

    let state_after: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let orders_after: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert_eq!(state_before, state_after);
    assert_eq!(orders_before, orders_after);
}

/// Critical atomicity case: a batch fills an existing resting order
/// (mutating the maker's position/margin, emitting `OrderFilled`), then
/// hits an invalid action. The fill itself must be rolled back — not
/// just the resting portion of the taker's message.
///
/// Maker user1 rests an ask. Taker user2 submits
/// [market buy that crosses, submit with duplicate cid]. The duplicate
/// cid fails the second action. After the revert, user1's resting ask
/// is still on the book and user1's UserState (margin, position) is
/// byte-identical to before the batch.
#[test]
fn batch_fill_reverts_on_later_failure() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // Both users deposit margin.
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

    // user1 rests an ask at $2,000 for 1 ETH.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(limit_ask(2_000, 1, None))),
            Coins::new(),
        )
        .should_succeed();

    // Snapshot maker (user1) and taker (user2) state plus their order books
    // before the batch. The taker-side snapshot catches a future bug that
    // would leak taker mutations (position, reserved_margin, unrested bids)
    // past the rollback.
    let user1_state_before: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let user1_orders_before: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let user2_state_before: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user2.address(),
        })
        .should_succeed()
        .unwrap();
    let user2_orders_before: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    // user2 submits a batch: market buy (crosses user1's ask) followed by a
    // submit whose duplicate client_order_id fails.
    let cid = 99u64;
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(SubmitOrderRequest {
                        pair_id: pair_id(),
                        size: Quantity::new_int(1),
                        kind: perps::OrderKind::Market {
                            max_slippage: Dimensionless::new_percent(50),
                        },
                        reduce_only: false,
                        tp: None,
                        sl: None,
                    }),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, Some(cid))),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_800, 1, Some(cid))),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_fail();

    // Both maker and taker state must be byte-identical — the fill against
    // user1's ask and any intermediate writes to user2 (margin reservation,
    // a bid resting mid-batch under cid=99) are rolled back along with the
    // rest of the batch.
    let user1_state_after: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let user1_orders_after: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let user2_state_after: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user2.address(),
        })
        .should_succeed()
        .unwrap();
    let user2_orders_after: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    assert_eq!(
        user1_state_before, user1_state_after,
        "maker UserState must be unchanged — the fill must revert"
    );
    assert_eq!(
        user1_orders_before, user1_orders_after,
        "maker's resting ask must still be on the book"
    );
    assert_eq!(
        user2_state_before, user2_state_after,
        "taker UserState must be unchanged — no position, margin, or \
         reserved_margin mutations may leak past the rollback"
    );
    assert_eq!(
        user2_orders_before, user2_orders_after,
        "taker must have no resting orders — the mid-batch bid at cid=99 \
         is rolled back along with the rest"
    );
}

/// Sanity: a single-action batch produces observable state equivalent
/// to the corresponding direct `SubmitOrder` message.
#[test]
fn batch_single_action() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![SubmitOrCancelOrderRequest::Submit(limit_bid(
                    1_900, 1, None,
                ))])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_succeed();

    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(orders.len(), 1);
}

/// `Param::max_action_batch_size` is enforced. Lower the cap to 3 via
/// `Configure`; a 4-action batch is rejected without touching any
/// storage.
#[test]
fn batch_size_cap_enforced() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // Lower the cap to 3.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    max_action_batch_size: 3,
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair_id() => default_pair_param(),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    let state_before: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    // 4-action batch exceeds the cap of 3 → rejected before any action runs.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_800, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_700, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_600, 1, None)),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_fail_with_error("exceeds maximum 3");

    let state_after: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    assert_eq!(state_before, state_after);

    // A 3-action batch at the cap still succeeds.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::BatchUpdateOrders(
                NonEmpty::new(vec![
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_900, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_800, 1, None)),
                    SubmitOrCancelOrderRequest::Submit(limit_bid(1_700, 1, None)),
                ])
                .unwrap(),
            )),
            Coins::new(),
        )
        .should_succeed();
}
