use {
    grug_app::{AppError, CHAIN_ID, CONFIG, CONTRACTS, GasTracker, TraceOption},
    grug_math::{Bytable, NextNumber, Uint128, Uint256},
    grug_testing::TestBuilder,
    grug_types::{
        Addr, BorshSerExt, Coins, Denom, Duration, Empty, JsonSerExt, MsgExecute, NextUpgrade,
        PastUpgrade, QuerierExt, ResultExt, StdError, Timestamp, btree_map, coins,
    },
    grug_vm_rust::ContractBuilder,
    std::str::FromStr,
};

#[test]
fn error_cases() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("jake", Coins::new())
        .set_owner("owner")
        .build();

    // Non-owner attempts to schedule an upgrade. Should fail.
    suite
        .upgrade(
            &mut accounts["jake"],
            3,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .should_fail_with_error(AppError::not_owner(
            accounts["jake"].address,
            accounts["owner"].address,
        ));

    // Block 2. Attempt to schedule upgrade at block 2. Should fail.
    suite
        .upgrade(
            &mut accounts["owner"],
            2,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .should_fail_with_error(AppError::upgrade_height_not_in_future(2, 2));
}

/// In this test, we attempt to change the chain ID through a chain upgrade.
/// This is otherwise not possible through normal transactions.
/// This upgrade doesn't involve any contract calls.
#[test]
fn upgrading_without_calling_contract() {
    const OLD_CHAIN_ID: &str = "oonga";
    const NEW_CHAIN_ID: &str = "boonga";

    let (mut suite, mut accounts) = TestBuilder::new()
        .set_chain_id(OLD_CHAIN_ID)
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .add_account("owner", Coins::new())
        .set_owner("owner")
        // .set_tracing_level(Some(tracing::Level::INFO)) // uncomment this to see tracing logs
        .build();

    // -------------------------------- Block 1 --------------------------------

    // Owner schedules an upgrade to happan at block 3. Upgrade doesn't happen yet.
    suite
        .upgrade(
            &mut accounts["owner"],
            3,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .should_succeed();

    suite.query_status().should_succeed_and(|status| {
        status.chain_id == OLD_CHAIN_ID && status.last_finalized_block.height == 1
    });

    suite
        .query_next_upgrade()
        .should_succeed_and_equal(Some(NextUpgrade {
            height: 3,
            cargo_version: "0.1.0".to_string(),
            git_tag: Some("v0.1.0".to_string()),
            url: Some("https://github.com".to_string()),
        }));

    // -------------------------------- Block 2 --------------------------------

    // Upgrade doesn't happen yet.
    suite.make_empty_block();

    suite.query_status().should_succeed_and(|status| {
        status.chain_id == OLD_CHAIN_ID && status.last_finalized_block.height == 2
    });

    // -------------------------------- Block 3 --------------------------------

    // Block 3. The chain halts as planned.
    suite
        .try_make_empty_block()
        .should_fail_with_error(AppError::upgrade_incorrect_version(
            "0.0.0".to_string(),
            "0.1.0".to_string(),
        ));

    // Perform the chain upgrade. Remove the halt height, add the upgrade handler.
    suite.app.set_cargo_version_and_upgrade_handler(
        "0.1.0",
        Some(|mut storage, _vm, _block| {
            CHAIN_ID.save(&mut storage, &NEW_CHAIN_ID.to_string())?;
            Ok(())
        }),
    );

    // Make block 3 again with the new app. Upgrade happens.
    suite.make_empty_block();

    suite.query_status().should_succeed_and(|status| {
        status.chain_id == NEW_CHAIN_ID && status.last_finalized_block.height == 3
    });

    // The next upgrade should have been removed.
    suite.query_next_upgrade().should_succeed_and_equal(None);

    // The upgrade history should have been saved.
    suite
        .query_past_upgrades(None, None)
        .should_succeed_and_equal(btree_map! {
            3 => PastUpgrade {
                cargo_version: "0.1.0".to_string(),
                git_tag: Some("v0.1.0".to_string()),
                url: Some("https://github.com".to_string()),
            },
        });
}

/// Do an upgrade that involves calling a contract.
///
/// In this test, we upgrade to an alternative bank contract where balances are
/// stored as 256 (instead of 128) bit numbers. During the upgrade, we need to
/// load all the 128-bit balances, convert them to 256-bit, and save.
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

        // NOTE: using `Uint256` here.
        pub const BALANCES_BY_ADDR: Map<(Addr, &Denom), Uint256> = Map::new("bu");

        /// Call this function to convert all balances to 256-bit.
        pub fn execute(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
            // Load all entries from the _old_ map and loop through them.
            // Do not confuse:
            // - The old map is `grug_mock_bank::BALANCES_BY_ADDR`.
            // - The new map is `new_mock_bank::BALANCES_BY_ADDR`.
            for ((addr, denom), amount) in grug_mock_bank::BALANCES_BY_ADDR
                .range(ctx.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?
            {
                // Convert the 128-bit amount to 256-bit.
                let amount = amount.into_next();

                // Save the 256-bit amount in the _new_ map.
                BALANCES_BY_ADDR.save(ctx.storage, (addr, &denom), &amount)?;
            }

            Ok(Response::new())
        }
    }

    let denom = Denom::from_str("oonga").unwrap();

    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", coins! { denom.clone() => 123 })
        .set_owner("owner")
        // .set_tracing_level(Some(tracing::Level::INFO)) // uncomment this to see tracing logs
        .build();

    let bank = suite.query_bank().unwrap();

    // -------------------------------- Block 1 --------------------------------

    // Owner schedules an upgrade to happan at block 3. Upgrade doesn't happen yet.
    suite
        .upgrade(
            &mut accounts["owner"],
            3,
            "0.1.0",
            None::<String>,
            None::<String>,
        )
        .should_succeed();

    // -------------------------------- Block 2 --------------------------------

    // Upgrade doesn't happen yet. The balance should be 128-bit.
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

    // -------------------------------- Block 3 --------------------------------

    // The chain halts as planned.
    suite
        .try_make_empty_block()
        .should_fail_with_error(AppError::upgrade_incorrect_version(
            "0.0.0".into(),
            "0.1.0".into(),
        ));

    // Perform the chain upgrade. Remove the halt height, add the upgrade handler.
    suite.app.set_cargo_version_and_upgrade_handler(
        "0.1.0",
        Some(|mut storage, vm, block| {
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
            .into_result()
            .map(|_| ())
            .map_err(|(_evt, err)| err)
        }),
    );

    // Make block 3 again with the new app. Upgrade happens.
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
