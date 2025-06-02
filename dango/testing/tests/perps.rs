use {
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        account::single,
        account_factory::AccountParams,
        constants::{dango, usdc},
        perps::{
            self, INITIAL_SHARES_PER_TOKEN, PerpsMarketParams, PerpsVaultState,
            QueryPerpsMarketParamsForDenomRequest, QueryPerpsVaultStateRequest,
            QueryVaultSharesForUserRequest,
        },
    },
    grug::{
        Coins, Denom, Int128, Message, QuerierExt, ResultExt, Udec128, Uint128, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    grug_vm_rust::VmError,
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
            realised_cash_flow: Default::default(),
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
            realised_cash_flow: Default::default(),
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
            realised_cash_flow: Default::default(),
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

    // Register prices
    register_fixed_prices(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let username = accounts.user1.username.clone();
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(username)),
            Coins::new(),
        )
        .should_succeed();

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

    // Register prices
    register_fixed_prices(&mut suite, &mut accounts, &contracts);

    // Create a margin account.
    let username = accounts.user1.username.clone();
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
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
            coins! { usdc::DENOM.clone() => 100_000_000, dango::DENOM.clone() => 100_000_000 },
        )
        .should_succeed();

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
        .should_fail_with_error("incorrect fee amount sent. sent: 99, expected: 200000");
}

#[test]
fn cant_open_position_if_trading_disabled() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

    // Register prices
    register_fixed_prices(&mut suite, &mut accounts, &contracts);

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

    // Create a margin account.
    let username = accounts.user1.username.clone();
    let mut margin_account = accounts
        .user1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Margin(single::Params::new(username)),
            Coins::new(),
        )
        .should_succeed();

    // Send some USDC to the margin account
    suite
        .transfer(
            &mut accounts.user1,
            margin_account.address.into_inner(),
            coins! { usdc::DENOM.clone() => 200_000 },
        )
        .should_succeed();

    // Try to open position, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.perps,
            &perps::ExecuteMsg::BatchUpdateOrders {
                orders: btree_map! { dango::DENOM.clone() => Int128::new(100_000_000) },
            },
            coins! { usdc::DENOM.clone() => 200_000},
        )
        .should_fail_with_error(
            "trading is not enabled for this market. you can only decrease your position size",
        );
}
