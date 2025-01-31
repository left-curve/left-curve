use {
    dango_testing::setup_test_naive,
    dango_types::{constants::USDC_DENOM, taxman},
    grug::{Addressable, Coins, MultiplyFraction, Number, ResultExt, Udec128, Uint128},
};

const OLD_FEE_RATE: Udec128 = Udec128::new_percent(1); // 0.01 uusdc per gas unit
const NEW_FEE_RATE: Udec128 = Udec128::new_percent(2); // 0.02 uusdc per gas unit

#[test]
fn fee_rate_update_works() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Starting balances of the two accounts
    let owner_usdc_balance = Uint128::new(100_000_000_000);
    let user_usdc_balance = Uint128::new(100_000_000_000_000);

    // --------------------------------- tx 1 ----------------------------------

    // At this point, the fee rate is zero.
    // Let's first set it to a non-zero value.
    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: USDC_DENOM.clone(),
                    fee_rate: OLD_FEE_RATE,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // This transaction was run when fee rate was zero.
    // The owner's USDC balance should be unchanged.
    suite
        .query_balance(&accounts.owner, USDC_DENOM.clone())
        .should_succeed_and_equal(owner_usdc_balance);

    // --------------------------------- tx 2 ----------------------------------

    // Owner makes another fee rate update.
    let success = suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: USDC_DENOM.clone(),
                    fee_rate: NEW_FEE_RATE,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // Owner should have been charged a gas fee, but at the old rate.
    let fee = Uint128::new(success.gas_used as u128)
        .checked_mul_dec_ceil(OLD_FEE_RATE)
        .unwrap();
    let _owner_usdc_balance = suite
        .query_balance(&accounts.owner, USDC_DENOM.clone())
        .should_succeed_and_equal(owner_usdc_balance.checked_sub(fee).unwrap());

    // --------------------------------- tx 3 ----------------------------------

    // Someone else sends a transaction.
    let success = suite
        .transfer(&mut accounts.user1, accounts.owner.address(), Coins::new())
        .should_succeed();

    // Gas fee should be calculated using the new rate.
    let fee = Uint128::new(success.gas_used as u128)
        .checked_mul_dec_ceil(NEW_FEE_RATE)
        .unwrap();
    let _user_usdc_balance = suite
        .query_balance(&accounts.user1, USDC_DENOM.clone())
        .should_succeed_and_equal(user_usdc_balance.checked_sub(fee).unwrap());
}
