use {
    grug_testing::TestBuilder,
    grug_types::{to_json_value, Binary, Coin, Coins, Empty, Message, NumberConst, Uint128},
    grug_vm_wasm::{VmError, WasmVm},
    std::{fs, io, vec},
};

const WASM_CACHE_CAPACITY: usize = 10;
const DENOM: &str = "ugrug";

fn read_wasm_file(filename: &str) -> io::Result<Binary> {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).map(Into::into)
}

#[test]
fn bank_transfers() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("sender", Coins::new_one(DENOM, 100_u128))?
        .add_account("receiver", Coins::new_empty())?
        .build()?;

    // Check that sender has been given 100 ugrug
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(100))?;

    // Sender sends 70 ugrug to the receiver across multiple messages
    suite
        .execute_messages(&accounts["sender"], 2_500_000, vec![
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 10_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 15_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 20_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: accounts["receiver"].address.clone(),
                coins: vec![Coin::new(DENOM, 25_u128)].try_into().unwrap(),
            },
        ])?
        .should_succeed()?;

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(30))?;
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(70))?;

    Ok(())
}

#[test]
fn gas_limit_too_low() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("sender", Coins::new_one(DENOM, 100_u128))?
        .add_account("receiver", Coins::new_empty())?
        .build()?;

    // Make a bank transfer with a small gas limit; should fail.
    // Bank transfers should take around ~500k gas.
    suite
        .execute_messages(&accounts["sender"], 100_000, vec![Message::Transfer {
            to: accounts["receiver"].address.clone(),
            coins: vec![Coin::new(DENOM, 10_u128)].try_into().unwrap(),
        }])?
        .should_fail_with_error(VmError::GasDepletion)?;

    // Tx is went out of gas.
    // Balances should remain the same
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint128::new(100))?;
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint128::ZERO)?;

    Ok(())
}

#[test]
fn infinite_loop() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("sender", Coins::new_one(DENOM, 100_u128))?
        .build()?;

    let tester_code = read_wasm_file("grug_tester_infinite_loop.wasm")?;
    let tester_salt = b"tester/infinite_loop".to_vec().into();
    let tester = suite.deploy_contract(
        &accounts["sender"],
        320_000_000,
        tester_code,
        tester_salt,
        &Empty {},
    )?;

    suite
        .execute_messages(&accounts["sender"], 1_000_000, vec![Message::Execute {
            contract: tester,
            msg: to_json_value(&Empty {})?,
            funds: Coins::new_empty(),
        }])?
        .should_fail_with_error(VmError::GasDepletion)?;

    Ok(())
}

#[test]
fn immutable_state() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("sender", Coins::new_one(DENOM, 100_u128))?
        .build()?;

    // Deploy the tester contract
    let tester_code = read_wasm_file("grug_tester_immutable_state.wasm")?;
    let tester_salt = b"tester/immutable_state".to_vec().into();
    let tester = suite.deploy_contract(
        &accounts["sender"],
        // Currently, deploying a contract consumes an exceedingly high amount
        // of gas because of the need to allocate hundreds ok kB of contract
        // bytecode into Wasm memory and have the contract deserialize it...
        320_000_000,
        tester_code,
        tester_salt,
        &Empty {},
    )?;

    // Query the tester contract.
    //
    // During the query, the contract attempts to write to the state by directly
    // calling the `db_write` import.
    //
    // This tests how the VM handles state mutability while serving the `Query`
    // ABCI request.
    suite
        .query_wasm_smart::<_, Empty>(tester.clone(), &Empty {})
        .should_fail_with_error(VmError::ReadOnly)?;

    // Execute the tester contract.
    //
    // During the execution, the contract makes a query to itself and the query
    // tries to write to the storage.
    //
    // This tests how the VM handles state mutability while serving the
    // `FinalizeBlock` ABCI request.
    suite
        .execute_messages(&accounts["sender"], 1_000_000, vec![Message::Execute {
            contract: tester,
            msg: to_json_value(&Empty {})?,
            funds: Coins::default(),
        }])?
        .should_fail_with_error(VmError::ReadOnly)?;

    Ok(())
}
