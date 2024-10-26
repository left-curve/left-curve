use std::{str::FromStr, sync::LazyLock};

use grug::{Addressable, MultiplyFraction, Number, Uint128};

use {
    dango_testing::setup_test,
    dango_types::taxman,
    grug::{Coins, Denom, ResultExt, Udec128},
};

const OLD_FEE_RATE: Udec128 = Udec128::new_percent(1); // 0.01 uusdc per gas unit
const NEW_FEE_RATE: Udec128 = Udec128::new_percent(2); // 0.02 uusdc per gas unit

static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

#[test]
fn fee_rate_update_works() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

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
                    fee_denom: USDC.clone(),
                    fee_rate: OLD_FEE_RATE,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // This transaction was run when fee rate was zero.
    // The owner's USDC balance shold be unchanged.
    suite
        .query_balance(&accounts.owner, USDC.clone())
        .should_succeed_and_equal(owner_usdc_balance);

    // --------------------------------- tx 2 ----------------------------------

    // Owner makes another fee rate update.
    let success = suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: USDC.clone(),
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
        .query_balance(&accounts.owner, USDC.clone())
        .should_succeed_and_equal(owner_usdc_balance.checked_sub(fee).unwrap());

    // --------------------------------- tx 3 ----------------------------------

    // Someone else sends a transaction.
    let success = suite
        .transfer(
            &mut accounts.relayer,
            accounts.owner.address(),
            Coins::new(),
        )
        .should_succeed();

    // Gas fee should be calculated using the new rate.
    let fee = Uint128::new(success.gas_used as u128)
        .checked_mul_dec_ceil(NEW_FEE_RATE)
        .unwrap();
    let _user_usdc_balance = suite
        .query_balance(&accounts.relayer, USDC.clone())
        .should_succeed_and_equal(user_usdc_balance.checked_sub(fee).unwrap());
}
