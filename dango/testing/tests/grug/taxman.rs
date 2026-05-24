use {
    dango_genesis::{AccountOption, GenesisOption, GenesisUser, TaxmanOption},
    dango_testing::{
        ContractBuilder, Preset, TestOption,
        constants::{owner, user1, user2, user3, user4, user5, user6, user7, user8, user9},
        setup_test_naive_with_custom_genesis,
    },
    dango_types::{account_factory::NewUserSalt, auth::Key},
    grug_math::{NumberConst, Uint128},
    grug_types::{Addressable, Coins, HashExt, Message, ResultExt},
    test_case::test_case,
};

/// The `RustVm` doesn't support gas metering, meaning the `TxOutcome::gas_used`
/// will always be zero (if the contract doesn't call any host function).
/// To do testing, we create this mock up taxman that will pretend that a
/// quarter of the gas limit was used and charge the fee accordingly.
mod mock_taxman {
    use {
        dango_types::constants::dango,
        grug_math::{IsZero, MultiplyFraction, Number, NumberConst, Udec128, Uint128},
        grug_types::{
            AuthCtx, AuthMode, Coins, Denom, Empty, Message, MutableCtx, QuerierExt, Response,
            StdResult, Tx, TxOutcome,
        },
        std::sync::LazyLock,
    };

    pub static FEE_DENOM: LazyLock<Denom> = LazyLock::new(|| dango::DENOM.clone());

    pub const FEE_RATE: Udec128 = Udec128::new_percent(25);

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
        let bank = ctx.querier.query_bank()?;

        // In simulation mode, don't do anything.
        if ctx.mode == AuthMode::Simulate {
            return Ok(Response::new());
        }

        let withhold_amount = Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(FEE_RATE)?;

        let withhold_msg = if withhold_amount.is_non_zero() {
            Some(Message::execute(
                bank,
                &dango_types::bank::ExecuteMsg::ForceTransfer {
                    from: tx.sender,
                    to: ctx.contract,
                    coins: Coins::one(FEE_DENOM.clone(), withhold_amount)?,
                },
                Coins::new(),
            )?)
        } else {
            None
        };

        Ok(Response::new().may_add_message(withhold_msg))
    }

    pub fn finalize_fee(ctx: AuthCtx, tx: Tx, _outcome: TxOutcome) -> StdResult<Response> {
        // In simulation mode, don't do anything.
        if ctx.mode == AuthMode::Simulate {
            return Ok(Response::new());
        }

        // We pretend that the tx used a quarter of the gas limit.
        let mock_gas_used = tx.gas_limit / 4;
        let withheld_amount = Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(FEE_RATE)?;
        let charge_amount = Uint128::new(mock_gas_used as u128).checked_mul_dec_ceil(FEE_RATE)?;
        let refund_amount = withheld_amount.saturating_sub(charge_amount);

        let charge_msg = if charge_amount.is_non_zero() {
            let owner = ctx.querier.query_owner()?;
            Some(Message::transfer(
                owner,
                Coins::one(FEE_DENOM.clone(), charge_amount)?,
            )?)
        } else {
            None
        };

        let refund_msg = if refund_amount.is_non_zero() {
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
    Some("subtraction overflow: 10 - 25000");
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
    Some("subtraction overflow: 5000 - 99999");
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
#[tokio::test]
async fn withholding_and_finalizing_fee_works(
    sender_balance_before: u128,
    send_amount: u128,
    gas_limit: u64,
    owner_balance_after: u128,
    sender_balance_after: u128,
    receiver_balance_after: u128,
    maybe_err: Option<&str>,
) {
    let taxman_code = ContractBuilder::new(Box::new(mock_taxman::instantiate))
        .with_withhold_fee(Box::new(mock_taxman::withhold_fee))
        .with_finalize_fee(Box::new(mock_taxman::finalize_fee))
        .build();

    let (mut suite, mut accounts, ..) = setup_test_naive_with_custom_genesis(
        TestOption {
            bridge_ops: |_| vec![],
            ..TestOption::default()
        },
        GenesisOption {
            taxman: TaxmanOption {
                alternative_code: Some(taxman_code.to_bytes().into()),
            },
            account: AccountOption {
                genesis_users: vec![
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(owner::PUBLIC_KEY.into()),
                            key_hash: owner::PUBLIC_KEY.hash256(),
                            seed: 0,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user1::PUBLIC_KEY.into()),
                            key_hash: user1::PUBLIC_KEY.hash256(),
                            seed: 1,
                        },
                        dango_balance: Uint128::new(sender_balance_before),
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user2::PUBLIC_KEY.into()),
                            key_hash: user2::PUBLIC_KEY.hash256(),
                            seed: 2,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user3::PUBLIC_KEY.into()),
                            key_hash: user3::PUBLIC_KEY.hash256(),
                            seed: 3,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user4::PUBLIC_KEY.into()),
                            key_hash: user4::PUBLIC_KEY.hash256(),
                            seed: 4,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user5::PUBLIC_KEY.into()),
                            key_hash: user5::PUBLIC_KEY.hash256(),
                            seed: 5,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user6::PUBLIC_KEY.into()),
                            key_hash: user6::PUBLIC_KEY.hash256(),
                            seed: 6,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user7::PUBLIC_KEY.into()),
                            key_hash: user7::PUBLIC_KEY.hash256(),
                            seed: 7,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user8::PUBLIC_KEY.into()),
                            key_hash: user8::PUBLIC_KEY.hash256(),
                            seed: 8,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                    GenesisUser {
                        salt: NewUserSalt {
                            key: Key::Secp256k1(user9::PUBLIC_KEY.into()),
                            key_hash: user9::PUBLIC_KEY.hash256(),
                            seed: 9,
                        },
                        dango_balance: Uint128::ZERO,
                    },
                ],
                ..Preset::preset_test()
            },
            ..Preset::preset_test()
        },
    );

    let to = accounts.user9.address();

    let outcome = suite
        .send_message_with_gas(
            &mut accounts.user1,
            gas_limit,
            Message::transfer(
                to,
                Coins::one(mock_taxman::FEE_DENOM.clone(), send_amount).unwrap(),
            )
            .unwrap(),
        )
        .await;

    match maybe_err {
        Some(err) => {
            outcome.should_fail_with_error(err);
        },
        None => {
            outcome.should_succeed();
        },
    }

    suite
        .query_balance(&accounts.owner, mock_taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(owner_balance_after));
    suite
        .query_balance(&accounts.user1, mock_taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(sender_balance_after));
    suite
        .query_balance(&accounts.user9, mock_taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(receiver_balance_after));
}

// In this test, we see what happens if the tx fails at the `finalize_fee` stage.
//
// This can be considered an "undefined behavior", because the taxman contract
// is supposed to be designed in a way such that `finalize_fee` never fails.
//
// If it does fail though, we simply discard all state changes and events emitted
// by the transaction, as if it never happened. We also print a log to the CLI
// at the ERROR tracing level to raise developer's awareness.
#[tokio::test]
async fn finalizing_fee_erroring() {
    let bugged_taxman_code = ContractBuilder::new(Box::new(mock_taxman::instantiate))
        .with_withhold_fee(Box::new(mock_taxman::withhold_fee))
        .with_finalize_fee(Box::new(mock_taxman::bugged_finalize_fee))
        .build();

    let (mut suite, mut accounts, ..) =
        setup_test_naive_with_custom_genesis(TestOption::default(), GenesisOption {
            taxman: TaxmanOption {
                alternative_code: Some(bugged_taxman_code.to_bytes().into()),
            },
            ..Preset::preset_test()
        });

    let sender_balance_before = suite
        .query_balance(&accounts.user1, mock_taxman::FEE_DENOM.clone())
        .unwrap();

    let owner_balance_before = suite
        .query_balance(&accounts.owner, mock_taxman::FEE_DENOM.clone())
        .unwrap();

    let to = accounts.user1.address();

    // Send a transaction with a single message.
    // `withhold_fee` must pass, which should be the case as we're requesting
    // a small gas limit.
    let outcome = suite
        .send_message_with_gas(
            &mut accounts.user1,
            2000,
            Message::transfer(to, Coins::new()).unwrap(),
        )
        .await;

    // Result should be an error.
    let failing = outcome.should_fail_with_error("division by zero: 1 / 0");

    // The finalize event should show the error.
    assert!(failing.events.finalize.maybe_error().is_some());

    // Owner and sender's balances shouldn't have changed, since state changes
    // are discarded.
    suite
        .query_balance(&accounts.owner, mock_taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(owner_balance_before);
    suite
        .query_balance(&accounts.user1, mock_taxman::FEE_DENOM.clone())
        .should_succeed_and_equal(sender_balance_before);
}
