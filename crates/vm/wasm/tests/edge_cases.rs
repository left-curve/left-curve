use {
    grug_app::AppError,
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{Addr, Binary, Coins, JsonSerExt, Message, NonZero, Udec128},
    grug_vm_wasm::{VmError, WasmVm},
    std::{fs, io, str::FromStr},
};

const WASM_CACHE_CAPACITY: usize = 10;
const DENOM: &str = "ugrug";
const FEE_RATE: &str = "0.1";

fn read_wasm_file(filename: &str) -> io::Result<Binary> {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).map(Into::into)
}

fn setup_test() -> anyhow::Result<(TestSuite<WasmVm>, TestAccounts, Addr)> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

    Ok((suite, accounts, tester))
}

#[test]
fn infinite_loop() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    suite
        .send_message_with_gas(&accounts["sender"], 1_000_000, Message::Execute {
            contract: tester,
            msg: grug_tester::ExecuteMsg::InfiniteLoop {}.to_json_value()?,
            funds: Coins::new(),
        })?
        .result
        .should_fail_with_error("out of gas");

    Ok(())
}

#[test]
fn immutable_state() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    // Query the tester contract.
    //
    // During the query, the contract attempts to write to the state by directly
    // calling the `db_write` import.
    //
    // This tests how the VM handles state mutability while serving the `Query`
    // ABCI request.
    suite
        .query_wasm_smart(tester, grug_tester::QueryForceWriteRequest {
            key: "larry".to_string(),
            value: "engineer".to_string(),
        })
        .should_fail_with_error(VmError::ImmutableState);

    // Execute the tester contract.
    //
    // During the execution, the contract makes a query to itself and the query
    // tries to write to the storage.
    //
    // This tests how the VM handles state mutability while serving the
    // `FinalizeBlock` ABCI request.
    suite
        .send_message_with_gas(&accounts["sender"], 1_000_000, Message::Execute {
            contract: tester,
            msg: grug_tester::ExecuteMsg::ForceWriteOnQuery {
                key: "larry".to_string(),
                value: "engineer".to_string(),
            }
            .to_json_value()?,
            funds: Coins::new(),
        })?
        .result
        .should_fail_with_error(VmError::ImmutableState);

    Ok(())
}

#[test]
fn query_stack_overflow() -> anyhow::Result<()> {
    let (suite, _, tester) = setup_test()?;

    // The contract attempts to call with `QueryMsg::StackOverflow` to itself in
    // a loop. Should raise the "exceeded max query depth" error.
    suite
        .query_wasm_smart(tester, grug_tester::QueryStackOverflowRequest {})
        .should_fail_with_error(VmError::ExceedMaxQueryDepth);

    Ok(())
}

#[test]
fn message_stack_overflow() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    // The contract attempts to return a Response with `Execute::StackOverflow`
    // to itself in a loop. Should raise the "exceeded max message depth" error.
    suite
        .send_message_with_gas(
            &accounts["sender"],
            10_000_000,
            Message::execute(
                tester,
                &grug_tester::ExecuteMsg::StackOverflow {},
                Coins::default(),
            )?,
        )?
        .result
        .should_fail_with_error(AppError::ExceedMaxMessageDepth);

    Ok(())
}
