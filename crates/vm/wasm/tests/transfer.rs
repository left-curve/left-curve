use {
    grug_testing::TestBuilder,
    grug_types::{
        Coins, Message, MultiplyFraction, NonZero, NumberConst, ResultExt, Udec128, Uint256,
    },
    grug_vm_wasm::WasmVm,
    std::{collections::BTreeMap, str::FromStr, vec},
};

const WASM_CACHE_CAPACITY: usize = 10;
const DENOM: &str = "ugrug";
const FEE_RATE: &str = "0.1";

#[test]
fn transfers() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(300_000_u128)?))?
        .add_account("receiver", Coins::new())?
        .set_owner("owner")?
        .set_fee_denom(DENOM)
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    // Check that sender has been given 300,000 ugrug.
    // Sender needs to have sufficient tokens to cover gas fee and the transfers.
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint256::from(300_000_u128));
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::ZERO);

    // Sender sends 70 ugrug to the receiver across multiple messages
    let outcome = suite.send_messages_with_gas(&accounts["sender"], 2_500_000, vec![
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(10_u128)?),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(15_u128)?),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(20_u128)?),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(25_u128)?),
        },
    ])?;

    outcome.result.should_succeed();

    // Sender remaining balance should be 300k - 70 - withhold + (withhold - charge).
    // = 300k - 70 - charge
    let fee = Uint256::from(outcome.gas_used).checked_mul_dec_ceil(Udec128::from_str(FEE_RATE)?)?;
    let sender_balance_after = Uint256::from(300_000_u128 - 70) - fee;

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(sender_balance_after);
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::from(70_u128));

    let info = suite.query_info().should_succeed();

    // List all holders of the denom
    suite
        .query_wasm_smart(info.config.bank, grug_bank::QueryHoldersRequest {
            denom: DENOM.to_string(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(BTreeMap::from([
            (accounts["owner"].address, fee),
            (accounts["sender"].address, sender_balance_after),
            (accounts["receiver"].address, Uint256::from(70_u128)),
        ]));

    Ok(())
}

#[test]
fn transfers_with_insufficient_gas_limit() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(200_000_u128)?))?
        .add_account("receiver", Coins::new())?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    // Make a bank transfer with a small gas limit; should fail.
    // Bank transfers should take around ~1M gas.
    //
    // We can't easily tell whether gas will run out during the Wasm execution
    // (in which case, the error would be a `VmError::GasDepletion`) or during
    // a host function call (in which case, a `VmError::OutOfGas`). We can only
    // say that the error has to be one of the two. Therefore, we simply ensure
    // the error message contains the word "gas".
    let outcome = suite.send_message_with_gas(&accounts["sender"], 100_000, Message::Transfer {
        to: accounts["receiver"].address,
        coins: Coins::one(DENOM, NonZero::new(10_u128)?),
    })?;

    outcome.result.should_fail();

    // The transfer should have failed, but gas fee already spent is still charged.
    let fee = Uint256::from(outcome.gas_used).checked_mul_dec_ceil(Udec128::from_str(FEE_RATE)?)?;
    let sender_balance_after = Uint256::from(200_000_u128) - fee;

    // Tx is went out of gas.
    // Balances should remain the same
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(sender_balance_after);
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::ZERO);

    Ok(())
}
