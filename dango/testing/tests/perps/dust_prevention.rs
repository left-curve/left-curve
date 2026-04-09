use {
    crate::{default_pair_param, default_param, register_oracle_prices},
    dango_genesis::{GenesisOption, PerpsOption},
    dango_testing::{Preset, TestOption, perps::pair_id, setup_test_naive_with_custom_genesis},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, PairParam, UserState},
    },
    grug::{Addressable, Coins, QuerierExt, ResultExt, Uint128, btree_map},
};

/// Build a genesis option with a custom `min_position_size` for the perps pair.
fn genesis_with_min_position_size(min_position_size: UsdValue) -> GenesisOption {
    let pair = pair_id();
    GenesisOption {
        perps: PerpsOption {
            param: default_param(),
            pair_params: btree_map! {
                pair => PairParam {
                    min_position_size,
                    ..default_pair_param()
                },
            },
        },
        ..GenesisOption::preset_test()
    }
}

/// A tiny market buy whose resulting notional ($20) is below
/// `min_position_size` ($100) is rejected.
#[test]
fn dust_order_rejected() {
    let genesis = genesis_with_min_position_size(UsdValue::new_int(100));
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(TestOption::default(), genesis);

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // Deposit margin.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Try to buy 0.01 ETH → notional = 0.01 * $2,000 = $20 < $100 min.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair_id(),
                size: Quantity::new_raw(10_000), // 0.01 ETH (6 decimals)
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("resulting position notional is below minimum");
}

/// Partially closing a position such that the remainder is below
/// `min_position_size` is rejected — even for reduce-only orders.
#[test]
fn partial_close_leaving_dust_rejected() {
    let genesis = genesis_with_min_position_size(UsdValue::new_int(100));
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(TestOption::default(), genesis);

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // --- Fund maker and trader ---

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
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

    // Open long 5 ETH ($10k notional, well above $100 min).
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
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Try to close 4.99 ETH → leaves 0.01 ETH = $20 < $100 min.
    // Place a bid for the maker to fill against.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_raw(-4_990_000), // -4.99 ETH
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("resulting position notional is below minimum");
}

/// A full close (resulting position = 0) always succeeds regardless of
/// `min_position_size`.
#[test]
fn full_close_always_allowed() {
    let genesis = genesis_with_min_position_size(UsdValue::new_int(100));
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(TestOption::default(), genesis);

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // --- Fund maker and trader ---

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Place ask for opening + bid for closing.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
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

    // Open long 5 ETH.
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
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Place bid for the close.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Full close: sell all 5 ETH → resulting position = 0, always allowed.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify position is gone.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    let state = state.unwrap();
    assert!(
        state.positions.is_empty(),
        "position should be fully closed"
    );
}
