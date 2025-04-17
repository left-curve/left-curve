use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::USDC_DENOM,
        oracle::{self, PriceSource},
        taxman::{self, FeePayments},
    },
    grug::{
        Addressable, Coins, MultiplyFraction, Number, NumberConst, QuerierExt, ResultExt, Udec128,
        Uint128, btree_map, coins,
    },
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

#[test]
fn query_fees_for_user_works() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Register oracle price source for USDC
    let precision = 6u32;
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: precision as u8,
                    timestamp: 1730802926,
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Query fees for user1, should be empty
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: None,
        })
        .should_succeed_and_equal(FeePayments::default());

    // Send a fee payment to the taxman
    let user1_addr = accounts.user1.address();
    let first_fees = coins! { USDC_DENOM.clone() => 100_000_000 };
    let first_fee_payments = FeePayments {
        coins: first_fees.clone(),
        usd_value: first_fees
            .amount_of(&USDC_DENOM)
            .checked_div(Uint128::new(10).checked_pow(6).unwrap())
            .unwrap(),
    };
    suite
        .execute(
            &mut accounts.user1,
            contracts.taxman,
            &taxman::ExecuteMsg::Pay {
                payments: btree_map! {
                    user1_addr => (taxman::FeeType::Gas, first_fees.clone()),
                },
            },
            first_fees.clone(),
        )
        .should_succeed();

    // Get current time
    let time_after_first_payment = suite.block.timestamp;

    // Query fees for user1, should be the fees paid
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: None,
        })
        .should_succeed_and_equal(first_fee_payments.clone());

    // Query fees for user1 for gas fee, should be the fees paid
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: Some(taxman::FeeType::Gas),
            since: None,
        })
        .should_succeed_and_equal(first_fee_payments.clone());

    // Query fees for user1 for other fee types, should be empty
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: Some(taxman::FeeType::Maker),
            since: None,
        })
        .should_succeed_and_equal(FeePayments::default());

    // Query fees for other user, should be empty
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user2.address(),
            fee_type: None,
            since: None,
        })
        .should_succeed_and_equal(FeePayments::default());

    // Query fees for user1 since the current time, should be empty
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: Some(time_after_first_payment),
        })
        .should_succeed_and_equal(FeePayments::default());

    // Make second fee payment with the same fee type
    suite
        .execute(
            &mut accounts.user1,
            contracts.taxman,
            &taxman::ExecuteMsg::Pay {
                payments: btree_map! { user1_addr => (taxman::FeeType::Gas, first_fees.clone()) },
            },
            first_fees.clone(),
        )
        .should_succeed();

    // Get current time
    let time_after_second_payment = suite.block.timestamp;

    // Query fees for user1 since the time after the first payment, should be the fees paid
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: Some(time_after_first_payment),
        })
        .should_succeed_and_equal(first_fee_payments.clone());

    // Query all fees for user1, should be the all the fees paid
    let second_fees = first_fees.clone();
    let mut expected_fees = first_fees.clone();
    expected_fees.insert_many(second_fees.clone()).unwrap();
    let expected_fee_payments = FeePayments {
        coins: expected_fees.clone(),
        usd_value: expected_fees
            .amount_of(&USDC_DENOM)
            .checked_div(Uint128::new(10).checked_pow(precision).unwrap())
            .unwrap(),
    };
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: None,
        })
        .should_succeed_and_equal(expected_fee_payments.clone());

    // Make third fee payment with a different fee type
    let third_fees = coins! { USDC_DENOM.clone() => 200_000_000 };
    let third_fee_payments = FeePayments {
        coins: third_fees.clone(),
        usd_value: third_fees
            .amount_of(&USDC_DENOM)
            .checked_div(Uint128::new(10).checked_pow(precision).unwrap())
            .unwrap(),
    };
    suite
        .execute(
            &mut accounts.user1,
            contracts.taxman,
            &taxman::ExecuteMsg::Pay {
                payments: btree_map! {
                    user1_addr => (taxman::FeeType::Maker, third_fees.clone()),
                },
            },
            third_fees.clone(),
        )
        .should_succeed();

    // Query gas fee for user1, should be same as the first two payments
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: Some(taxman::FeeType::Gas),
            since: None,
        })
        .should_succeed_and_equal(expected_fee_payments.clone());

    // Query maker fee for user1, should be the third payment
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: Some(taxman::FeeType::Maker),
            since: None,
        })
        .should_succeed_and_equal(third_fee_payments.clone());

    // Query all fees for user1, should be the all the fees paid
    let mut expected_fees = first_fees.clone();
    expected_fees.insert_many(second_fees).unwrap();
    expected_fees.insert_many(third_fees.clone()).unwrap();
    let expected_fee_payments = FeePayments {
        coins: expected_fees.clone(),
        usd_value: expected_fees
            .amount_of(&USDC_DENOM)
            .checked_div(Uint128::new(10).checked_pow(precision).unwrap())
            .unwrap(),
    };
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: None,
        })
        .should_succeed_and_equal(expected_fee_payments.clone());

    // Query maker fee for user1 after the second payment, should be the third payment
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: Some(taxman::FeeType::Maker),
            since: Some(time_after_second_payment),
        })
        .should_succeed_and_equal(third_fee_payments.clone());

    // Get current time
    let time_after_third_payment = suite.block.timestamp;

    // Query all fees for user1 since the time after the third payment, should be empty as no new fees were paid
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryFeesForUserRequest {
            user: accounts.user1.address(),
            fee_type: None,
            since: Some(time_after_third_payment),
        })
        .should_succeed_and_equal(FeePayments::default());
}
