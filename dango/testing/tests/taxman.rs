use {
    dango_testing::setup_test_naive,
    dango_types::{constants::usdc, taxman},
    grug::{
        Addressable, BalanceChange, Coins, Inner, MultiplyFraction, ResultExt, Udec128, Uint128,
        btree_map,
    },
};

const OLD_FEE_RATE: Udec128 = Udec128::new_percent(1); // 0.01 uusdc per gas unit
const NEW_FEE_RATE: Udec128 = Udec128::new_percent(2); // 0.02 uusdc per gas unit

#[test]
fn fee_rate_update_works() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // --------------------------------- tx 1 ----------------------------------

    suite.balances().record(&accounts.owner);

    // At this point, the fee rate is zero.
    // Let's first set it to a non-zero value.
    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: usdc::DENOM.clone(),
                    fee_rate: OLD_FEE_RATE,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // This transaction was run when fee rate was zero.
    // The owner's USDC balance should be unchanged.
    suite.balances().should_change(&accounts.owner, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });

    // --------------------------------- tx 2 ----------------------------------

    suite.balances().record(&accounts.owner);

    // Owner makes another fee rate update.
    let success = suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: usdc::DENOM.clone(),
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

    suite.balances().should_change(&accounts.owner, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Decreased(fee.into_inner()),
    });

    // --------------------------------- tx 3 ----------------------------------

    suite.balances().record(&accounts.user1);

    // Someone else sends a transaction.
    let success = suite
        .transfer(&mut accounts.user1, accounts.owner.address(), Coins::new())
        .should_succeed();

    // Gas fee should be calculated using the new rate.
    let fee = Uint128::new(success.gas_used as u128)
        .checked_mul_dec_ceil(NEW_FEE_RATE)
        .unwrap();

    suite.balances().should_change(&accounts.user1, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Decreased(fee.into_inner()),
    });
}
