use {
    dango_genesis::Contracts,
    dango_testing::{TestAccount, TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        account::single,
        account_factory::AccountParams,
        constants::{dango, usdc},
        perps::{
            self, INITIAL_SHARES_PER_TOKEN, PerpsMarketAccumulators, PerpsMarketParams,
            PerpsMarketState, PerpsPositionResponse, PerpsVaultState, Pnl,
            QueryPerpsMarketParamsForDenomRequest, QueryPerpsMarketStateForDenomRequest,
            QueryPerpsPositionsForUserRequest, QueryPerpsPositionsRequest,
            QueryPerpsVaultStateRequest, QueryVaultSharesForUserRequest,
        },
    },
    grug::{
        Coins, Dec128, Denom, Duration, Int128, Message, NumberConst, QuerierExt, ResultExt, Sign,
        Signed, Udec128, Uint128, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    grug_vm_rust::VmError,
    std::str::FromStr,
};

/// Helper function to register a fixed price for a collateral
fn register_fixed_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    denom: Denom,
    humanized_price: Udec128,
    precision: u8,
) {
    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                denom => dango_types::oracle::PriceSource::Fixed {
                    humanized_price,
                    precision,
                    timestamp: 0,
                }
            }),
            Coins::default(),
        )
        .should_succeed();
}

/// Registers fixed prices for USDC and DANGO
fn register_fixed_prices(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    // Register fixed price for USDC
    register_fixed_price(
        suite,
        accounts,
        contracts,
        usdc::DENOM.clone(),
        Udec128::new_percent(100),
        6,
    );

    // Register fixed price for DANGO
    register_fixed_price(
        suite,
        accounts,
        contracts,
        dango::DENOM.clone(),
        Udec128::new_percent(100),
        6,
    );
}

/// Creates a new margin account and sends some funds to it.
fn create_margin_account(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> TestAccount {
    let username = accounts.user1.username.clone();
    let margin_account = accounts
        .user1
        .register_new_account(
            suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(username)),
            Coins::new(),
        )
        .should_succeed();

    // Send some USDC and DANGO to the margin account
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address.into_inner(),
            coins! { usdc::DENOM.clone() => 1_000_000_000, dango::DENOM.clone() => 1_000_000_000 },
        )
        .should_succeed();

    margin_account
}

/// Registers fixed prices and creates a single margin account.
fn perps_test_setup(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> TestAccount {
    // Register prices
    register_fixed_prices(suite, accounts, contracts);

    // Create a margin account.
    create_margin_account(suite, accounts, contracts)
}

#[test]
fn cant_transfer_to_perps() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    suite
        .send_message(
            &mut accounts.user1,
            Message::transfer(contracts.perps, coins! { usdc::DENOM.clone() => 123 }).unwrap(),
        )
        .should_fail_with_error(VmError::function_not_found("receive"));
}

#[test]
fn cant_deposit_wrong_denom() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    // Try to deposit with wrong denom, should fail
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            coins! {
                dango::DENOM.clone() => 123,
            },
        )
        .should_fail_with_error("invalid payment: expecting bridge/usdc, found dango");

    // Try to deposit with wrong and correct denom, should fail
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            coins! {
                usdc::DENOM.clone() => 123,
                dango::DENOM.clone() => 123,
            },
        )
        .should_fail_with_error("invalid payment: expecting 1, found 2");
}

#[test]
fn deposit_works() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    // Register prices
    register_fixed_prices(&mut suite, &mut accounts, &contracts);

    // Deposit 123 USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            coins! {
                usdc::DENOM.clone() => 123,
            },
        )
        .should_succeed();

    // Ensure the perps vault state is updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsVaultStateRequest {})
        .should_succeed_and_equal(PerpsVaultState {
            denom: usdc::DENOM.clone(),
            deposits: Uint128::from(123),
            shares: Uint128::from(123) * INITIAL_SHARES_PER_TOKEN,
            realised_pnl: Default::default(),
        });

    // Ensure the user's deposit is updated
    suite
        .query_wasm_smart(contracts.perps, QueryVaultSharesForUserRequest {
            address: accounts.user1.address.into_inner(),
        })
        .should_succeed_and_equal(Uint128::from(123) * INITIAL_SHARES_PER_TOKEN);
}

#[test]
fn withdraw_works() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    // Register prices
    register_fixed_prices(&mut suite, &mut accounts, &contracts);

    // Deposit 123 USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            coins! { usdc::DENOM.clone() => 123 },
        )
        .should_succeed();

    // Ensure the perps vault state is updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsVaultStateRequest {})
        .should_succeed_and_equal(PerpsVaultState {
            denom: usdc::DENOM.clone(),
            deposits: Uint128::from(123),
            shares: Uint128::from(123) * INITIAL_SHARES_PER_TOKEN,
            realised_pnl: Default::default(),
        });

    // Ensure the user's deposit is updated
    suite
        .query_wasm_smart(contracts.perps, QueryVaultSharesForUserRequest {
            address: accounts.user1.address.into_inner(),
        })
        .should_succeed_and_equal(Uint128::from(123) * INITIAL_SHARES_PER_TOKEN);

    // Withdraw 100 USDC
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Withdraw {
                shares: Uint128::from(100) * INITIAL_SHARES_PER_TOKEN,
            },
            Coins::new(),
        )
        .should_succeed();

    // Ensure the perps vault state is updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsVaultStateRequest {})
        .should_succeed_and_equal(PerpsVaultState {
            denom: usdc::DENOM.clone(),
            deposits: Uint128::from(23),
            shares: Uint128::from(23) * INITIAL_SHARES_PER_TOKEN,
            realised_pnl: Default::default(),
        });

    // Ensure the user's deposit is updated
    suite
        .query_wasm_smart(contracts.perps, QueryVaultSharesForUserRequest {
            address: accounts.user1.address.into_inner(),
        })
        .should_succeed_and_equal(Uint128::from(23) * INITIAL_SHARES_PER_TOKEN);
}

#[test]
fn only_margin_accounts_can_update_orders() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    // Try to open position, should fail
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100) },
            },
            Coins::new(),
        )
        .should_fail_with_error("only margin accounts can update orders");
}

#[test]
fn cant_open_position_with_too_small_size() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Try to open position with too small size, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100) },
            },
            Coins::new(),
        )
        .should_fail_with_error("position size is too small");
}

#[test]
fn cant_open_position_without_correct_fee() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Try to open position without funds, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            Coins::new(),
        )
        .should_fail_with_error("invalid payment: expecting 1, found 0");

    // Try to open position with wrong denom, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { dango::DENOM.clone() => 100 },
        )
        .should_fail_with_error("invalid payment: expecting bridge/usdc, found dango");

    // Try to open position with wrong amount, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 99 },
        )
        .should_fail_with_error("incorrect fee amount sent. sent: 99, expected: 202000");
}

#[test]
fn cant_open_position_if_trading_disabled() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Set DANGO market trading disabled
    let params = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketParamsForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed();
    suite.execute(
        &mut accounts.owner,
        contracts.perps,
        &perps::ExecuteMsg::UpdatePerpsMarketParams {
            params: btree_map! { dango::DENOM.clone() => PerpsMarketParams { trading_enabled: false, ..params } },
        },
        Coins::new(),
    ).should_succeed();

    // Try to open position, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 202_000},
        )
        .should_fail_with_error(
            "trading is not enabled for this market. you can only decrease your position size",
        );
}

#[test]
fn cant_open_position_that_would_exceed_max_oi() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Try to open too big long position, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(1_000_000_001) },
            },
            coins! { usdc::DENOM.clone() => 2_200_000 },
        )
        .should_fail_with_error("position size would exceed max long oi");

    // Try to open too big short position, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(-1_000_000_001) },
            },
            coins! { usdc::DENOM.clone() => 1_800_000 },
        )
        .should_fail_with_error("position size would exceed max short oi");
}

#[test]
fn open_position_works() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Open position
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 202_000 },
        )
        .should_succeed();

    let oracle_price = Udec128::from_str("0.000001").unwrap();

    // Ensure the perps positions are updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account.address.into_inner(),
        })
        .should_succeed_and_equal(btree_map! { dango::DENOM.clone() => PerpsPositionResponse {
            denom: dango::DENOM.clone(),
            size: Int128::new(100_000_000),
            entry_skew: Int128::ZERO,
            realized_pnl: Pnl {
                fees: Int128::new(-202_000),
                price_pnl: Int128::ZERO,
                funding_pnl: Int128::ZERO,
            },
            entry_funding_index: Dec128::ZERO,
            entry_execution_price: Dec128::from_str("0.00000101").unwrap(),
            entry_price: oracle_price,
            unrealized_pnl: Pnl {
                fees: Int128::new(-202_000),
                price_pnl: Int128::ZERO,
                funding_pnl: Int128::ZERO,
            }
        } });
}

#[test]
fn vault_pnl_works() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    let mut margin_account = perps_test_setup(&mut suite, &mut accounts, &contracts);

    // Open position
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 202_000 },
        )
        .should_succeed();

    let oracle_price = Udec128::from_str("0.000001").unwrap();

    let margin_account_1_positions = suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account.address.into_inner(),
        })
        .should_succeed_and_equal(btree_map! { dango::DENOM.clone() => PerpsPositionResponse {
            denom: dango::DENOM.clone(),
            size: Int128::new(100_000_000),
            entry_skew: Int128::ZERO,
            realized_pnl: Pnl {
                fees: Int128::new(-202_000),
                price_pnl: Int128::ZERO,
                funding_pnl: Int128::ZERO,
            },
            entry_funding_index: Dec128::ZERO,
            entry_execution_price: Dec128::from_str("0.00000101").unwrap(),
            entry_price: oracle_price,
            unrealized_pnl: Pnl {
                fees: Int128::new(-202_000),
                price_pnl: Int128::ZERO,
                funding_pnl: Int128::ZERO,
            }
        } });

    // Ensure the perps market state is updated
    let perps_market_state = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketStateForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed_and_equal(PerpsMarketState {
            denom: dango::DENOM.clone(),
            long_oi: Uint128::from(100_000_000),
            short_oi: Uint128::ZERO,
            last_updated: suite.block.timestamp,
            last_funding_rate: Dec128::ZERO,
            last_funding_index: Dec128::ZERO,
            accumulators: PerpsMarketAccumulators {
                cost_basis_sum: Dec128::new(100_000_000) * Dec128::from_str("0.00000101").unwrap(),
                funding_basis_sum: Dec128::ZERO,
                quadratic_fee_basis: Int128::new(100_000_000) * Int128::new(100_000_000),
            },
            realised_pnl: Pnl {
                fees: Int128::new(202_000),
                ..Default::default()
            },
        });
    // Ensure skew is correct
    let skew = perps_market_state.skew().unwrap();
    assert_eq!(skew, Int128::new(100_000_000));

    // Ensure market PnL is correct
    let params = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketParamsForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed();
    let unrealized_price_pnl = perps_market_state
        .unrealized_price_pnl(oracle_price, params.skew_scale)
        .unwrap();
    assert_eq!(unrealized_price_pnl, Int128::ZERO);
    let unrealized_funding_pnl = perps_market_state.unrealized_funding_pnl().unwrap();
    assert_eq!(unrealized_funding_pnl, Int128::ZERO);

    // Market PnL should just be the realized pnl, since unrealized PnL is 0
    let market_pnl = perps_market_state
        .market_pnl(&params, oracle_price)
        .unwrap();
    assert_eq!(market_pnl, perps_market_state.realised_pnl.total().unwrap());

    // Increase time of chain so that funding rate accrues
    suite.increase_time(Duration::from_days(30));

    // Open second position in opposite direction
    let mut margin_account_2 = create_margin_account(&mut suite, &mut accounts, &contracts);
    suite
        .execute(
            &mut margin_account_2,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(-100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 202_000 },
        )
        .should_succeed();

    // Ensure the perps positions are updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and(|positions| {
            assert_eq!(positions.len(), 2);

            // Margin account 1 position should be unchanged except for unrealized funding pnl
            let margin_account_1_new_positions =
                positions.get(&margin_account.address.into_inner()).unwrap();
            assert_eq!(margin_account_1_new_positions.len(), 1);
            let margin_account_1_new_position = margin_account_1_new_positions
                .get(&dango::DENOM.clone())
                .unwrap();
            let margin_account_1_old_position = margin_account_1_positions
                .get(&dango::DENOM.clone())
                .unwrap();
            assert_eq!(
                margin_account_1_new_position.size,
                margin_account_1_old_position.size
            );
            assert_eq!(
                margin_account_1_new_position.entry_skew,
                margin_account_1_old_position.entry_skew
            );
            assert_eq!(
                margin_account_1_new_position.realized_pnl,
                margin_account_1_old_position.realized_pnl
            );
            assert_eq!(
                margin_account_1_new_position.entry_price,
                margin_account_1_old_position.entry_price
            );
            assert_eq!(
                margin_account_1_new_position.entry_execution_price,
                margin_account_1_old_position.entry_execution_price
            );
            assert_eq!(
                margin_account_1_new_position.entry_funding_index,
                margin_account_1_old_position.entry_funding_index
            );
            assert_eq!(
                margin_account_1_new_position.unrealized_pnl.fees,
                margin_account_1_old_position.unrealized_pnl.fees
            );
            assert_eq!(
                margin_account_1_new_position.unrealized_pnl.price_pnl,
                margin_account_1_old_position.unrealized_pnl.price_pnl
            );
            assert!(
                margin_account_1_new_position.unrealized_pnl.funding_pnl
                    < margin_account_1_old_position.unrealized_pnl.funding_pnl
            );

            let margin_account_2_positions = positions
                .get(&margin_account_2.address.into_inner())
                .unwrap();
            assert_eq!(margin_account_2_positions.len(), 1);
            let marign_account_2_position = margin_account_2_positions
                .get(&dango::DENOM.clone())
                .unwrap();
            assert_eq!(marign_account_2_position.size, Int128::new(-100_000_000));
            assert_eq!(
                marign_account_2_position.entry_skew,
                Int128::new(100_000_000)
            );
            assert_eq!(marign_account_2_position.realized_pnl, Pnl {
                fees: Int128::new(-202_000),
                price_pnl: Int128::ZERO,
                funding_pnl: Int128::ZERO,
            });
            assert_eq!(marign_account_2_position.entry_price, oracle_price);
            true
        });

    // Ensure the perps market state is updated
    let perps_market_state = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketStateForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed_and(|state| {
            assert_eq!(state.long_oi, Uint128::from(100_000_000));
            assert_eq!(state.short_oi, Uint128::from(100_000_000));
            assert_eq!(state.last_updated, suite.block.timestamp);
            assert_eq!(state.accumulators.cost_basis_sum, Dec128::ZERO);
            assert_eq!(state.accumulators.quadratic_fee_basis, Int128::ZERO);
            assert_eq!(state.realised_pnl.fees, Int128::new(404_000));
            assert_eq!(state.realised_pnl.price_pnl, Int128::ZERO);
            assert_eq!(state.realised_pnl.funding_pnl, Int128::ZERO);
            true
        });
    // Ensure market skew is correct
    let skew = perps_market_state.skew().unwrap();
    assert_eq!(skew, Int128::ZERO);

    // Ensure market PnL is correct
    let params = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketParamsForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed();
    let market_unrealized_price_pnl = perps_market_state
        .unrealized_price_pnl(oracle_price, params.skew_scale)
        .unwrap();
    assert_eq!(market_unrealized_price_pnl, Int128::ZERO);
    let market_unrealized_funding_pnl = perps_market_state.unrealized_funding_pnl().unwrap();
    assert!(market_unrealized_funding_pnl > Int128::ZERO);
    let market_pnl = perps_market_state
        .market_pnl(&params, oracle_price)
        .unwrap();
    // Market PnL should be the realized pnl plus the unrealized funding pnl
    assert_eq!(
        market_pnl,
        perps_market_state.realised_pnl.total().unwrap() + market_unrealized_funding_pnl
    );

    // Increase time of chain
    suite.increase_time(Duration::from_days(30));

    // Query the market state after increasing time
    let perps_market_state = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketStateForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed();

    // Query the first position
    let margin_account_1_positions = suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account.address.into_inner(),
        })
        .should_succeed();
    let margin_account_1_position = margin_account_1_positions
        .get(&dango::DENOM.clone())
        .unwrap();

    // Close first position
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(-100_000_000) },
            },
            coins! { usdc::DENOM.clone() => margin_account_1_position.unrealized_pnl.fees.unsigned_abs() },
        )
        .should_succeed();

    // Ensure the perps positions are updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account.address.into_inner(),
        })
        .should_succeed_and_equal(btree_map! {}); // Position should be deleted from storage

    // Ensure the perps market state is updated
    let perps_market_state = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketStateForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed_and(|state| {
            assert_eq!(state.long_oi, Uint128::ZERO);
            assert_eq!(state.short_oi, Uint128::new(100_000_000));
            assert_eq!(state.last_updated, suite.block.timestamp);
            assert_eq!(
                state.accumulators.cost_basis_sum,
                Dec128::new(-100_000_000) * Dec128::from_str("0.00000101").unwrap(),
            );
            assert_eq!(
                state.accumulators.quadratic_fee_basis,
                Int128::new(-100_000_000) * Int128::new(100_000_000)
            );
            assert_eq!(
                state.realised_pnl.fees,
                perps_market_state.realised_pnl.fees
                    + margin_account_1_position
                        .unrealized_pnl
                        .fees
                        .checked_neg()
                        .unwrap()
            );
            assert_eq!(
                state.realised_pnl.price_pnl,
                margin_account_1_position
                    .unrealized_pnl
                    .price_pnl
                    .checked_neg()
                    .unwrap()
            );
            assert_eq!(
                state.realised_pnl.funding_pnl,
                perps_market_state.realised_pnl.funding_pnl
                    + margin_account_1_position
                        .unrealized_pnl
                        .funding_pnl
                        .checked_neg()
                        .unwrap()
            );
            true
        });

    // Query the second position
    let margin_account_2_positions = suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account_2.address.into_inner(),
        })
        .should_succeed();
    let margin_account_2_position = margin_account_2_positions
        .get(&dango::DENOM.clone())
        .unwrap();

    // Close second position
    suite
        .execute(
            &mut margin_account_2,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => margin_account_2_position.unrealized_pnl.fees.unsigned_abs() },
        )
        .should_succeed();

    // Ensure the perps positions are updated
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsPositionsForUserRequest {
            address: margin_account_2.address.into_inner(),
        })
        .should_succeed_and_equal(btree_map! {}); // Position should be deleted from storage

    // Ensure the perps market state is updated
    let perps_market_state = suite
        .query_wasm_smart(contracts.perps, QueryPerpsMarketStateForDenomRequest {
            denom: dango::DENOM.clone(),
        })
        .should_succeed_and(|state| {
            assert_eq!(state.long_oi, Uint128::ZERO);
            assert_eq!(state.short_oi, Uint128::ZERO);
            assert_eq!(state.last_updated, suite.block.timestamp);
            assert_eq!(state.accumulators.cost_basis_sum, Dec128::ZERO,);
            assert_eq!(state.accumulators.quadratic_fee_basis, Int128::ZERO,);
            assert_eq!(
                state.realised_pnl.fees,
                perps_market_state.realised_pnl.fees
                    + margin_account_2_position
                        .unrealized_pnl
                        .fees
                        .checked_neg()
                        .unwrap()
            );
            assert_eq!(
                state.realised_pnl.price_pnl,
                perps_market_state.realised_pnl.price_pnl
                    + margin_account_2_position
                        .unrealized_pnl
                        .price_pnl
                        .checked_neg()
                        .unwrap()
            );
            assert_eq!(
                state.realised_pnl.funding_pnl,
                perps_market_state.realised_pnl.funding_pnl
                    + margin_account_2_position
                        .unrealized_pnl
                        .funding_pnl
                        .checked_neg()
                        .unwrap()
            );
            true
        });

    // Query the perps vault state
    suite
        .query_wasm_smart(contracts.perps, QueryPerpsVaultStateRequest {})
        .should_succeed_and(|state| {
            assert_eq!(
                state.realised_pnl.fees,
                perps_market_state.realised_pnl.fees
            );
            assert_eq!(
                state.realised_pnl.price_pnl,
                perps_market_state.realised_pnl.price_pnl
            );
            assert_eq!(
                state.realised_pnl.funding_pnl,
                perps_market_state.realised_pnl.funding_pnl
            );
            true
        });
}
