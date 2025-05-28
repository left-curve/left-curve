use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::usdc,
        perps::{
            self, INITIAL_SHARES_PER_TOKEN, PerpsVaultState, QueryPerpsVaultStateRequest,
            QueryVaultSharesForUserRequest,
        },
    },
    grug::{Coins, Message, QuerierExt, ResultExt, Uint128, coins},
    grug_vm_rust::VmError,
};

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
fn deposit_works() {
    let (mut suite, mut accounts, _codes, contracts, _) = setup_test_naive(Default::default());

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
