use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Empty, Message, NumberConst, ResultExt, TxOutcome, Uint256},
    grug_vm_rust::ContractBuilder,
    test_case::test_case,
};

/// The `RustVm` doesn't support gas metering, meaning the `TxOutcome::gas_used`
/// will always be zero (if the contract doesn't call any host function).
/// To do testing, we create this mock up taxman that will pretend that a
/// quarter of the gas limit was used and charge the fee accordingly.
mod taxman {
    use {
        grug_types::{
            AuthCtx, AuthMode, Coins, Denom, Empty, Message, MultiplyFraction, MutableCtx, Number,
            NumberConst, Response, StdResult, Tx, TxOutcome, Udec128, Uint128, Uint256,
        },
        std::{str::FromStr, sync::LazyLock},
    };

    pub static FEE_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("ugrug").unwrap());

    pub static FEE_RATE: LazyLock<Udec128> = LazyLock::new(|| Udec128::from_str("0.25").unwrap());

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
        let cfg = ctx.querier.query_config()?;

        // In simulation mode, don't do anything.
        if ctx.mode == AuthMode::Simulate {
            return Ok(Response::new());
        }

        let withhold_amount = Uint256::from(tx.gas_limit).checked_mul_dec_ceil(*FEE_RATE)?;

        let withhold_msg = if !withhold_amount.is_zero() {
            Some(Message::execute(
                cfg.bank,
                &grug_bank::ExecuteMsg::ForceTransfer {
                    from: tx.sender,
                    to: ctx.contract,
                    denom: FEE_DENOM.clone(),
                    amount: withhold_amount,
                },
                Coins::new(),
            )?)
        } else {
            None
        };

        Ok(Response::new().may_add_message(withhold_msg))
    }

    pub fn finalize_fee(ctx: AuthCtx, tx: Tx, _outcome: TxOutcome) -> StdResult<Response> {
        let cfg = ctx.querier.query_config()?;

        // In simulation mode, don't do anything.
        if ctx.mode == AuthMode::Simulate {
            return Ok(Response::new());
        }

        // We pretend that the tx used a quarter of the gas limit.
        let mock_gas_used = tx.gas_limit / 4;
        let withheld_amount = Uint128::from(tx.gas_limit).checked_mul_dec_ceil(*FEE_RATE)?;
        let charge_amount = Uint128::from(mock_gas_used).checked_mul_dec_ceil(*FEE_RATE)?;
        let refund_amount = withheld_amount.saturating_sub(charge_amount);

        let charge_msg = if !charge_amount.is_zero() {
            Some(Message::transfer(
                cfg.owner,
                Coins::one(FEE_DENOM.clone(), charge_amount)?,
            )?)
        } else {
            None
        };

        let refund_msg = if !refund_amount.is_zero() {
            Some(Message::transfer(
                tx.sender,
                Coins::one(FEE_DENOM.clone(), refund_amount)?,
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
    pub fn bugged_finalize_fee(_ctx: AuthCtx, _tx: Tx, _outcome: TxOutcome) -> StdResult<Response> {
        let _ = Uint128::ONE.checked_div(Uint128::ZERO)?;

        Ok(Response::new())
    }
}

// In this test, a sender attempts to make a token transfer with various gas
// limit and transfer amounts.
//
// Depending on these variables, the transaction may fail either during
// `withhold_fee` or during processing the message.
//
// We check the transaction outcome and the account balances afterwards to make
// sure they are the expected values.
//
// Case 1. Sender has enough balance to make the transfer, but not enough to
// cover gas fee.
// The tx should fail at `withhold_fee` stage.
// No state change should be committed.
#[test_case(
    10,
    1,
    100_000,
    0,
    10,
    0,
    Some("subtraction overflow: 10 - 25000 < bnum::buint::BUint<4>::MIN");
    "error while withholding fee"
)]
// Case 2. Sender has enough balance to cover gas fee, but not enough for the
// transfer.
// The tx should pass `withhold_fee`, but fail at processing messages.
// The fee should be deducted from the sender's account, but the transfer reverted.
#[test_case(
    30_000,
    99_999,
    100_000,
    6250,  // = 100,000 / 4 * 0.25
    23750, // = 30,000 - (100,000 / 4 * 0.25)
    0,
    Some("subtraction overflow: 5000 - 99999 < bnum::buint::BUint<4>::MIN");
    "error while processing messages"
)]
// Case 3. Sender has enough balance to cover both gas fee and the transfer.
// State changes from both gas fee and transfer should be affected.
#[test_case(
    30_000,
    123,
    100_000,
    6250,  // = 100,000 / 4 * 0.25
    23627, // = 30,000 - (100,000 / 4 * 0.25) - 123
    123,
    None;
    "successful tx"
)]
fn withholding_and_finalizing_fee_works(
    sender_balance_before: u128,
    send_amount: u128,
    gas_limit: u64,
    owner_balance_after: u128,
    sender_balance_after: u128,
    receiver_balance_after: u128,
    maybe_err: Option<&str>,
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
            Coins::one(taxman::FEE_DENOM.clone(), sender_balance_before).unwrap(),
        )
        .unwrap()
        .add_account("receiver", Coins::new())
        .unwrap()
        .set_owner("owner")
        .unwrap()
        .build()
        .unwrap();

    let outcome = suite
        .send_message_with_gas(
            &accounts["sender"],
            gas_limit,
            Message::transfer(
                accounts["receiver"].address,
                Coins::one(taxman::FEE_DENOM.clone(), send_amount).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

    match maybe_err {
        Some(err) => {
            outcome.result.should_fail_with_error(err);
        },
        None => {
            outcome.result.should_succeed();
        },
    }

    suite
        .query_balance(&accounts["owner"], taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(owner_balance_after.into());
    suite
        .query_balance(&accounts["sender"], taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(sender_balance_after.into());
    suite
        .query_balance(&accounts["receiver"], taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(receiver_balance_after.into());
}

// In this test, we see what happens if the tx fails at the `finalize_fee` stage.
//
// This can be considered an "undefined behavior", because the taxman contract
// is supposed to be designed in a way such that `finalize_fee` never fails.
//
// If it does fail though, we simply discard all state changes and events emitted
// by the transaction, as if it never happened. We also print a log to the CLI
// at the ERROR tracing level to raise developer's awareness.
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
            Coins::one(taxman::FEE_DENOM.clone(), 30_000_u128).unwrap(),
        )
        .unwrap()
        .set_owner("owner")
        .unwrap()
        .build()
        .unwrap();

    // Send a transaction with a single message.
    // `withhold_fee` must pass, which should be the case as we're requesting
    // zero gas limit.
    let TxOutcome { events, result, .. } = suite
        .send_message_with_gas(
            &accounts["sender"],
            0,
            Message::transfer(accounts["sender"].address, Coins::new()).unwrap(),
        )
        .unwrap();

    // Result should be an error.
    result.should_fail_with_error("division by zero: 1 / 0");

    // All events should have been discarded.
    assert!(events.is_empty());

    // Owner and sender's balances shouldn't have changed, since state changes
    // are discarded.
    suite
        .query_balance(&accounts["owner"], taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(Uint256::ZERO);
    suite
        .query_balance(&accounts["sender"], taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(30_000_u128.into());
}
