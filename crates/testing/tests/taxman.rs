use {
    grug_testing::TestBuilder,
    grug_types::{
        Coins, Empty, Event, GenericResult, Message, MultiplyFraction, NonZero, Udec128, Uint128,
    },
    grug_vm_rust::ContractBuilder,
    std::str::FromStr,
    test_case::test_case,
};

/// The `RustVm` doesn't support gas metering, meaning the `TxOutcome::gas_used`
/// will always be zero (if the contract doesn't call any host function).
/// To do testing, we create this mock up taxman that will pretend that a
/// quarter of the gas limit was used and charge the fee accordingly.
mod taxman {
    use {
        grug_types::{
            Coins, Empty, Message, MultiplyFraction, MutableCtx, NonZero, Number, NumberConst,
            Outcome, Response, StdResult, SudoCtx, Tx, Udec128, Uint128,
        },
        std::str::FromStr,
    };

    pub const FEE_DENOM: &str = "ugrug";
    pub const FEE_RATE: &str = "0.25";

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn withhold_fee(ctx: SudoCtx, tx: Tx) -> StdResult<Response> {
        let info = ctx.querier.query_info()?;

        let fee_rate = Udec128::from_str(FEE_RATE)?;
        let withhold_amount = Uint128::from(tx.gas_limit).checked_mul_dec_ceil(fee_rate)?;

        let withhold_msg = if !withhold_amount.is_zero() {
            Some(Message::execute(
                info.config.bank,
                &grug_bank::ExecuteMsg::ForceTransfer {
                    from: tx.sender,
                    to: ctx.contract,
                    denom: FEE_DENOM.to_string(),
                    amount: withhold_amount,
                },
                Coins::new(),
            )?)
        } else {
            None
        };

        Ok(Response::new().may_add_message(withhold_msg))
    }

    pub fn finalize_fee(ctx: SudoCtx, tx: Tx, _outcome: Outcome) -> StdResult<Response> {
        let info = ctx.querier.query_info()?;

        // We pretend that the tx used a quarter of the gas limit.
        let mock_gas_used = tx.gas_limit / 4;
        let fee_rate = Udec128::from_str(FEE_RATE)?;
        let withheld_amount = Uint128::from(tx.gas_limit).checked_mul_dec_ceil(fee_rate)?;
        let charge_amount = Uint128::from(mock_gas_used).checked_mul_dec_ceil(fee_rate)?;
        let refund_amount = withheld_amount.saturating_sub(charge_amount);

        let charge_msg = if !charge_amount.is_zero() {
            let owner = info.config.owner.expect("owner not set");
            Some(Message::transfer(
                owner,
                Coins::one(FEE_DENOM, NonZero::new(charge_amount)),
            )?)
        } else {
            None
        };

        let refund_msg = if !refund_amount.is_zero() {
            Some(Message::transfer(
                tx.sender,
                Coins::one(FEE_DENOM, NonZero::new(refund_amount)),
            )?)
        } else {
            None
        };

        Ok(Response::new()
            .may_add_message(charge_msg)
            .may_add_message(refund_msg))
    }

    /// An alternative version of the `finalize_fee` function that errors on
    /// purpose. Used to test whether the `App` can correctly handle the case
    /// where `finalize_fee` errors.
    pub fn bugged_finalize_fee(_ctx: SudoCtx, _tx: Tx, _outcome: Outcome) -> StdResult<Response> {
        let _ = Uint128::ONE.checked_div(Uint128::ZERO)?;

        Ok(Response::new())
    }
}

/// A contract throws error on demand.
mod thrower {
    use grug_types::{Empty, MutableCtx, Number, NumberConst, Response, StdResult, Uint128};

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn execute(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn bugged_execute(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        let _ = Uint128::ONE.checked_div(Uint128::ZERO)?;

        Ok(Response::new())
    }
}

/// Like `TxOutcome`, but the specific events are omitted
struct ExpectedOutcome {
    withhold_fee_result: Option<GenericResult<()>>,
    process_msgs_result: Option<GenericResult<()>>,
    finalize_fee_result: Option<GenericResult<()>>,
}

fn assert_outcome(actual: &Option<GenericResult<Vec<Event>>>, expect: &Option<GenericResult<()>>) {
    match (actual, expect) {
        (None, None) | (Some(GenericResult::Ok(_)), Some(GenericResult::Ok(_))) => {
            // Good, nothing to do.
        },
        (Some(GenericResult::Err(err1)), Some(GenericResult::Err(err2))) => {
            // Check that the error match.
            assert!(
                err1.contains(err2),
                "errors don't match! actual: {err1}, expect: {err2}"
            );
        },
        _ => {
            panic!("outcomes mismatch! actual = {actual:?}, expect = {expect:?}");
        },
    }
}

#[test_case(
    // Sender doesn't have enough balance to cover fee.
    // 100,000 (gas limit) * 0.25 (fee rate) = 25,000 > 10
    Uint128::new(10),
    100_000,
    false,
    ExpectedOutcome {
        withhold_fee_result: Some(GenericResult::Err("subtraction overflow: 10 - 25000 < u128::MIN".to_string())),
        process_msgs_result: None,
        finalize_fee_result: None,
    };
    "error while withholding fee"
)]
#[test_case(
    Uint128::new(30_000),
    100_000,
    true,
    ExpectedOutcome {
        withhold_fee_result: Some(GenericResult::Ok(())),
        process_msgs_result: Some(GenericResult::Err("division by zero: 1 / 0".to_string())),
        finalize_fee_result: Some(GenericResult::Ok(())),
    };
    "error while processing messages"
)]
#[test_case(
    Uint128::new(30_000),
    100_000,
    false,
    ExpectedOutcome {
        withhold_fee_result: Some(GenericResult::Ok(())),
        process_msgs_result: Some(GenericResult::Ok(())),
        finalize_fee_result: Some(GenericResult::Ok(())),
    };
    "successful tx"
)]
fn withholding_and_finalizing_fee_works(
    sender_balance_before: Uint128,
    gas_limit: u64,
    throw: bool,
    expect: ExpectedOutcome,
) {
    let taxman_code = ContractBuilder::new(Box::new(taxman::instantiate))
        .with_withhold_fee(Box::new(taxman::withhold_fee))
        .with_finalize_fee(Box::new(taxman::finalize_fee))
        .build();

    let (mut suite, accounts) = TestBuilder::new()
        .set_taxman_code(taxman_code, |_fee_denom, _fee_rate| Empty {})
        .add_account("owner", Coins::new())
        .unwrap()
        .add_account(
            "sender",
            Coins::one(taxman::FEE_DENOM, NonZero::new(sender_balance_before)),
        )
        .unwrap()
        .set_owner("owner")
        .unwrap()
        .build()
        .unwrap();

    // Deploy the thrower contract.
    let thrower_code = ContractBuilder::new(Box::new(thrower::instantiate))
        .with_execute(if throw {
            Box::new(thrower::bugged_execute)
        } else {
            Box::new(thrower::execute)
        })
        .build();
    let (_, thrower) = suite
        .upload_and_instantiate(
            &accounts["owner"],
            thrower_code,
            "thrower",
            &Empty {},
            Coins::new(),
        )
        .unwrap();

    // Sender interacts with the thrower contract.
    let actual = suite
        .send_message_with_gas(
            &accounts["sender"],
            gas_limit,
            Message::execute(thrower, &Empty {}, Coins::new()).unwrap(),
        )
        .unwrap();

    // Tx outcome should match the expected value.
    assert_outcome(&actual.withhold_fee_result, &expect.withhold_fee_result);
    assert_outcome(&actual.process_msgs_result, &expect.process_msgs_result);
    assert_outcome(&actual.finalize_fee_result, &expect.finalize_fee_result);

    // Make sure the fee has been deducted from the sender's balance.
    // this should happen regardless of whether the messages succeeded or not.
    if let Some(GenericResult::Ok(_)) = expect.finalize_fee_result {
        let fee_amount = Uint128::from(gas_limit / 4)
            .checked_mul_dec_ceil(Udec128::from_str(taxman::FEE_RATE).unwrap())
            .unwrap();
        let sender_balance_after = sender_balance_before - fee_amount;

        suite
            .query_balance(&accounts["owner"], taxman::FEE_DENOM)
            .should_succeed_and_equal(fee_amount);
        suite
            .query_balance(&accounts["sender"], taxman::FEE_DENOM)
            .should_succeed_and_equal(sender_balance_after);
    }
}

#[test]
fn finalizing_fee_erroring() {
    let bugged_taxman_code = ContractBuilder::new(Box::new(taxman::instantiate))
        .with_withhold_fee(Box::new(taxman::withhold_fee))
        .with_finalize_fee(Box::new(taxman::bugged_finalize_fee))
        .build();

    let (mut suite, accounts) = TestBuilder::new()
        .set_taxman_code(bugged_taxman_code, |_fee_denom, _fee_rate| Empty {})
        .add_account("owner", Coins::new())
        .unwrap()
        .add_account(
            "sender",
            Coins::one(taxman::FEE_DENOM, NonZero::new(30_000_u128)),
        )
        .unwrap()
        .set_owner("owner")
        .unwrap()
        .build()
        .unwrap();

    // Sender attempts to send a transaction.
    let outcome = suite
        .send_message(
            &accounts["sender"],
            Message::transfer(accounts["sender"].address.clone(), Coins::new()).unwrap(),
        )
        .unwrap();

    assert_outcome(&outcome.withhold_fee_result, &None);
    assert_outcome(&outcome.process_msgs_result, &None);
    assert_outcome(
        &outcome.finalize_fee_result,
        &Some(GenericResult::Err("division by zero: 1 / 0".to_string())),
    );

    // Sender's balance shouldn't have changed, since state changes are discarded.
    suite
        .query_balance(&accounts["sender"], taxman::FEE_DENOM)
        .should_succeed_and_equal(Uint128::new(30_000));
}
