use {
    crate::{default_pair_param, default_param, register_oracle_prices},
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, Param},
    },
    grug::{Addressable, Coins, NumberConst, QuerierExt, ResultExt, Uint128, btree_map},
};

/// Drive a couple of fills with `protocol_fee_rate = 20%` to seed the treasury,
/// then exercise `MaintainerMsg::WithdrawFromTreasury`. Verify the treasury
/// drains to zero, the owner's `UserState.margin` gains the previous treasury
/// balance, and a follow-up `TraderMsg::Withdraw` round-trips USDC out of the
/// contract.
#[test]
fn treasury_withdrawal_works() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Configure: set protocol_fee_rate = 20%.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    protocol_fee_rate: Dimensionless::new_percent(20),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => default_pair_param(),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Deposit margin for maker (user2) and taker (user1).
    for user in [&mut accounts.user2, &mut accounts.user1] {
        suite
            .execute(
                user,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
                Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
            )
            .should_succeed();
    }

    // Two fills of 10 ETH @ $2,000 each. Per-fill fee = 10 * $2,000 * 0.1% = $20,
    // protocol cut = 20% = $4. After both fills, treasury == $8.
    for _ in 0..2 {
        suite
            .execute(
                &mut accounts.user2,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(
                    perps::SubmitOrderRequest {
                        pair_id: pair.clone(),
                        size: Quantity::new_int(-10),
                        kind: perps::OrderKind::Limit {
                            limit_price: UsdPrice::new_int(2_000),
                            time_in_force: perps::TimeInForce::PostOnly,
                            client_order_id: None,
                        },
                        reduce_only: false,
                        tp: None,
                        sl: None,
                    },
                )),
                Coins::new(),
            )
            .should_succeed();

        suite
            .execute(
                &mut accounts.user1,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(
                    perps::SubmitOrderRequest {
                        pair_id: pair.clone(),
                        size: Quantity::new_int(10),
                        kind: perps::OrderKind::Market {
                            max_slippage: Dimensionless::new_percent(50),
                        },
                        reduce_only: false,
                        tp: None,
                        sl: None,
                    },
                )),
                Coins::new(),
            )
            .should_succeed();
    }

    let pre_state: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();
    assert_eq!(
        pre_state.treasury,
        UsdValue::new_int(8),
        "expected $8 in treasury before withdrawal"
    );

    let owner_addr = accounts.owner.address();
    let pre_owner_margin = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: owner_addr,
        })
        .should_succeed()
        .map(|us: perps::UserState| us.margin)
        .unwrap_or(UsdValue::ZERO);

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::WithdrawFromTreasury {}),
            Coins::new(),
        )
        .should_succeed();

    // Treasury drained.
    let post_state: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();
    assert_eq!(post_state.treasury, UsdValue::ZERO);

    // Owner's UserState margin gained the previous treasury balance.
    let post_owner_state: perps::UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: owner_addr,
        })
        .should_succeed()
        .expect("owner should now have a UserState");
    assert_eq!(
        post_owner_state.margin,
        pre_owner_margin.checked_add(pre_state.treasury).unwrap(),
    );

    // Round-trip: owner can convert the credited margin back to USDC via the
    // standard withdraw path.
    suite.balances().record(&accounts.owner);
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Withdraw {
                amount: pre_state.treasury,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Owner should have received the USDC backing the treasury.
    let owner_post_balance = suite
        .query_balance(&accounts.owner, usdc::DENOM.clone())
        .unwrap();
    assert!(
        owner_post_balance > Uint128::ZERO,
        "owner should have non-zero USDC after round-trip withdraw"
    );
}

/// Non-owner callers must be rejected, regardless of treasury state.
#[test]
fn treasury_withdrawal_rejects_non_owner() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::WithdrawFromTreasury {}),
            Coins::new(),
        )
        .should_fail_with_error("you don't have the right");
}
