use {
    grug_app::{CHAIN_ID, CONFIG, CONTRACTS, GasTracker, TraceOption},
    grug_math::{Bytable, NextNumber, Uint128, Uint256},
    grug_testing::TestBuilder,
    grug_types::{
        Addr, BorshSerExt, Coins, Denom, Duration, Empty, JsonSerExt, MsgExecute, QuerierExt,
        ResultExt, StdError, Timestamp, coins,
    },
    grug_vm_rust::ContractBuilder,
    std::str::FromStr,
};

/// In this test, we attempt to change the chain ID through a chain upgrade.
/// This is otherwise not possible through normal transactions.
/// This upgrade doesn't involve any contract calls.
#[test]
fn upgrading_without_calling_contract() {
    const OLD_CHAIN_ID: &str = "oonga";
    const NEW_CHAIN_ID: &str = "boonga";

    let (mut suite, _) = TestBuilder::new()
        .set_chain_id(OLD_CHAIN_ID)
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .add_account("owner", Coins::new())
        .set_owner("owner")
        .set_upgrade_handler(3, |mut storage, _vm, _block| {
            CHAIN_ID.save(&mut storage, &NEW_CHAIN_ID.to_string())?;
            Ok(())
        })
        .build();

    // Block 1. Upgrade doesn't happen yet.
    suite.make_empty_block();
    suite.query_status().should_succeed_and(|status| {
        status.chain_id == OLD_CHAIN_ID && status.last_finalized_block.height == 1
    });

    // Block 2. Upgrade doesn't happen yet.
    suite.make_empty_block();
    suite.query_status().should_succeed_and(|status| {
        status.chain_id == OLD_CHAIN_ID && status.last_finalized_block.height == 2
    });

    // Block 3. Upgrade happens.
    suite.make_empty_block();
    suite.query_status().should_succeed_and(|status| {
        status.chain_id == NEW_CHAIN_ID && status.last_finalized_block.height == 3
    });
}

/// Do an upgrade that involves changing a contract's internal state.
///
/// In this test, we assume there is an alternative bank contract where balances
/// are stored as 256 (instead of 128) bit numbers. During the upgrade, we need
/// to load all the 128-bit balances, convert them to 256-bit, and save.
///
/// In practice, this can be easily done with a simple contract migration, but
/// here for testing purpose, we go with a full chain upgrade.
#[test]
fn upgrading_with_calling_contract() {
    mod new_mock_bank {
        use {
            grug_math::{NextNumber, Uint256},
            grug_storage::Map,
            grug_types::{Addr, Denom, Empty, MutableCtx, Order, Response, StdResult},
        };

        pub const BALANCES_BY_ADDR: Map<(Addr, &Denom), Uint256> = Map::new("bu");

        /// Call this function to convert all balances to 256-bit.
        pub fn execute(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
            // Loop through all the balances and convert them to 256-bit.
            for ((addr, denom), amount) in grug_mock_bank::BALANCES_BY_ADDR
                .range(ctx.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?
            {
                let amount = amount.into_next();
                BALANCES_BY_ADDR.save(ctx.storage, (addr, &denom), &amount)?;
            }

            Ok(Response::new())
        }
    }

    let denom = Denom::from_str("oonga").unwrap();

    let (mut suite, accounts) = TestBuilder::new()
        .set_upgrade_handler(3, |mut storage, vm, block| {
            let cfg = CONFIG.load(&storage)?;
            let bank_contract = CONTRACTS.load(&storage, cfg.bank)?;

            // Build the new bank contract code.
            let new_mock_bank_code = ContractBuilder::new(Box::new(grug_mock_bank::instantiate))
                .with_execute(Box::new(new_mock_bank::execute))
                .build();

            // Update the bank contract's code.
            grug_app::CODES.update(&mut storage, bank_contract.code_hash, |mut code| {
                code.code = new_mock_bank_code.to_bytes().into();
                Ok::<_, StdError>(code)
            })?;

            // Call the bank contract's `execute` function to update the balances
            // to 256-bit.
            grug_app::do_execute(
                vm,
                storage,
                GasTracker::new_limitless(),
                block,
                0,
                Addr::mock(0),
                MsgExecute {
                    contract: cfg.bank,
                    msg: (Empty {}).to_json_value()?,
                    funds: Coins::new(),
                },
                TraceOption::LOUD,
            )
            .as_result()
            .map(|_| ())
            .map_err(|(_evt, err)| err)
        })
        .add_account("owner", coins! { denom.clone() => 123 })
        .set_owner("owner")
        .build();

    let bank = suite.query_bank().unwrap();

    // Blocks 1 and 2. Upgrade doesn't happen yet.
    // The balance should be 128-bit.
    for _ in 1..=2 {
        suite.make_empty_block();
        suite
            .query_wasm_raw(
                bank,
                grug_mock_bank::BALANCES_BY_ADDR.path((accounts["owner"].address, &denom)),
            )
            .should_succeed_and(|bytes| {
                let bytes = bytes.as_ref().unwrap();
                bytes.len() == Uint128::BYTE_LEN
                    && bytes.as_ref() == Uint128::new(123).to_borsh_vec().unwrap()
            });
    }

    // Block 3. Upgrade happens.
    // The balance should be 256-bit now.
    suite.make_empty_block();
    suite
        .query_wasm_raw(
            bank,
            new_mock_bank::BALANCES_BY_ADDR.path((accounts["owner"].address, &denom)),
        )
        .should_succeed_and(|bytes| {
            let bytes = bytes.as_ref().unwrap();
            bytes.len() == Uint256::BYTE_LEN
                && bytes.as_ref() == Uint128::new(123).into_next().to_borsh_vec().unwrap()
        });
}
