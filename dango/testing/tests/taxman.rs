use {
    dango_testing::setup_test_naive,
    dango_types::{constants::usdc, taxman},
    grug::{
        Addressable, BalanceChange, Coins, Inner, IsZero, MultiplyFraction, NumberConst, ResultExt,
        Udec128, Uint128, btree_map,
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

const FEE_RATE: Udec128 = Udec128::new_percent(1);

#[test]
fn withdraw_fees_succeeds_for_owner() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set a non-zero fee rate so subsequent txs accumulate fees in taxman.
    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman::Config {
                    fee_denom: usdc::DENOM.clone(),
                    fee_rate: FEE_RATE,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // Run a few transfer txs from a non-owner so taxman accumulates gas fees.
    for _ in 0..3 {
        suite
            .transfer(&mut accounts.user1, accounts.user2.address(), Coins::new())
            .should_succeed();
    }

    let collected = suite
        .query_balance(&contracts.taxman, usdc::DENOM.clone())
        .unwrap();
    assert!(
        collected.is_non_zero(),
        "expected some fees to have been collected"
    );

    suite.balances().record(&accounts.owner);

    // Withdraw — owner pays gas for this tx, but the rest of taxman's USDC
    // ends up at the owner's address.
    let success = suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::WithdrawFees {},
            Coins::new(),
        )
        .should_succeed();

    let withdraw_gas_fee = Uint128::new(success.gas_used as u128)
        .checked_mul_dec_ceil(FEE_RATE)
        .unwrap();

    // The owner should have received `collected`, minus the gas fee paid for
    // this withdraw tx.
    suite.balances().should_change(&accounts.owner, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Increased(
            (collected - withdraw_gas_fee).into_inner(),
        ),
    });

    // After this tx, taxman should hold only the gas fee that the withdraw tx
    // itself just paid — everything previously collected is gone.
    let remaining = suite
        .query_balance(&contracts.taxman, usdc::DENOM.clone())
        .unwrap();
    assert_eq!(remaining, withdraw_gas_fee);
}

#[test]
fn withdraw_fees_rejects_non_owner() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    suite
        .execute(
            &mut accounts.user1,
            contracts.taxman,
            &taxman::ExecuteMsg::WithdrawFees {},
            Coins::new(),
        )
        .should_fail_with_error("you don't have the right");
}

#[test]
fn withdraw_fees_noop_when_empty() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Genesis fee rate is zero, and no other state lands in taxman during
    // setup — this withdraw tx itself is also gas-free for the same reason.
    let pre = suite
        .query_balance(&contracts.taxman, usdc::DENOM.clone())
        .unwrap();
    assert_eq!(pre, Uint128::ZERO);

    suite.balances().record(&accounts.owner);

    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::WithdrawFees {},
            Coins::new(),
        )
        .should_succeed();

    suite.balances().should_change(&accounts.owner, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });

    let post = suite
        .query_balance(&contracts.taxman, usdc::DENOM.clone())
        .unwrap();
    assert_eq!(post, Uint128::ZERO);
}
