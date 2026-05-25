use {
    dango_testing::{TestOption, setup_test_naive},
    dango_types::constants::usdc,
    grug_app::{AppError, CHAIN_ID, CONFIG, CONTRACTS, GasTracker, HaltReason, TraceOption},
    grug_math::{Bytable, NextNumber, Uint128, Uint256},
    grug_types::{
        Addr, Addressable, BorshSerExt, Coins, Duration, Empty, JsonSerExt, MsgExecute,
        NextUpgrade, PastUpgrade, QuerierExt, ResultExt, StdError, btree_map,
    },
    grug_vm_rust::ContractBuilder,
    std::sync::Arc,
};

#[tokio::test]
async fn error_cases() {
    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption::default());

    // Non-owner attempts to schedule an upgrade. Should fail.
    suite
        .upgrade(
            &mut accounts.user1,
            3,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .await
        .should_fail_with_error(AppError::not_owner(
            accounts.user1.address(),
            accounts.owner.address(),
        ));

    // Block 2. Attempt to schedule upgrade at block 2. Should fail.
    suite
        .upgrade(
            &mut accounts.owner,
            2,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .await
        .should_fail_with_error(AppError::upgrade_height_not_in_future(2, 2));
}

/// In this test, we attempt to change the chain ID through a chain upgrade.
/// This is otherwise not possible through normal transactions.
/// This upgrade doesn't involve any contract calls.
#[tokio::test]
async fn upgrading_without_calling_contract() {
    const NEW_CHAIN_ID: &str = "boonga";

    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption {
        block_time: Duration::from_seconds(1),
        ..TestOption::default()
    });

    let old_chain_id = suite.chain_id.clone();

    // -------------------------------- Block 1 --------------------------------

    // Owner schedules an upgrade to happan at block 3. Upgrade doesn't happen yet.
    suite
        .upgrade(
            &mut accounts.owner,
            3,
            "0.1.0",
            Some("v0.1.0"),
            Some("https://github.com"),
        )
        .await
        .should_succeed();

    suite.query_status().should_succeed_and(|status| {
        status.chain_id == old_chain_id && status.last_finalized_block.height == 1
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
    suite.make_empty_block().await;

    suite.query_status().should_succeed_and(|status| {
        status.chain_id == old_chain_id && status.last_finalized_block.height == 2
    });

    // -------------------------------- Block 3 --------------------------------

    // Block 3. The chain halts as planned.
    suite
        .try_make_empty_block()
        .await
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
    suite.make_empty_block().await;

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
#[tokio::test]
async fn upgrading_with_calling_contract() {
    mod new_bank {
        use {
            grug_math::{NextNumber, Uint256},
            grug_storage::Map,
            grug_types::{Addr, Denom, Empty, MutableCtx, Order, Response, StdResult},
        };

        // NOTE: using `Uint256` here instead of `Uint128`.
        pub const BALANCES_256: Map<(&Addr, &Denom), Uint256> = Map::new("balance");

        /// Call this function to convert all balances to 256-bit.
        pub fn execute(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
            // Load all entries from the _old_ map and loop through them.
            // The old map is `dango_bank::BALANCES` (Uint128).
            // The new map is `new_bank::BALANCES_256` (Uint256).
            // They share the same storage key "balance" but different value types.
            for ((addr, denom), amount) in dango_bank::BALANCES
                .range(ctx.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?
            {
                // Convert the 128-bit amount to 256-bit.
                let amount = amount.into_next();

                // Save the 256-bit amount in the _new_ map.
                BALANCES_256.save(ctx.storage, (&addr, &denom), &amount)?;
            }

            Ok(Response::new())
        }
    }

    let denom = usdc::DENOM.clone();

    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption::default());

    let bank = suite.query_bank().unwrap();

    // Record the owner's current USDC balance.
    let owner_balance = suite.query_balance(&accounts.owner, denom.clone()).unwrap();

    // -------------------------------- Block 1 --------------------------------

    // Owner schedules an upgrade to happan at block 3. Upgrade doesn't happen yet.
    suite
        .upgrade(
            &mut accounts.owner,
            3,
            "0.1.0",
            None::<String>,
            None::<String>,
        )
        .await
        .should_succeed();

    // -------------------------------- Block 2 --------------------------------

    // Upgrade doesn't happen yet. The balance should be 128-bit.
    suite.make_empty_block().await;

    suite
        .query_wasm_raw(
            bank,
            dango_bank::BALANCES.path((&accounts.owner.address(), &denom)),
        )
        .should_succeed_and(|bytes| {
            let bytes = bytes.as_ref().unwrap();
            bytes.len() == Uint128::BYTE_LEN
                && bytes.as_ref() == owner_balance.to_borsh_vec().unwrap()
        });

    // -------------------------------- Block 3 --------------------------------

    // The chain halts as planned.
    suite
        .try_make_empty_block()
        .await
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

            // Build the new bank contract code with our custom execute that
            // converts 128-bit balances to 256-bit.
            let new_bank_code = ContractBuilder::new(Box::new(dango_bank::instantiate))
                .with_execute(Box::new(new_bank::execute))
                .build();

            // Update the bank contract's code.
            grug_app::CODES.update(&mut storage, bank_contract.code_hash, |mut code| {
                code.code = new_bank_code.to_bytes().into();
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
    suite.make_empty_block().await;

    suite
        .query_wasm_raw(
            bank,
            new_bank::BALANCES_256.path((&accounts.owner.address(), &denom)),
        )
        .should_succeed_and(|bytes| {
            let bytes = bytes.as_ref().unwrap();
            bytes.len() == Uint256::BYTE_LEN
                && bytes.as_ref() == owner_balance.into_next().to_borsh_vec().unwrap()
        });
}

/// When the chain reaches a scheduled upgrade height but the running binary's
/// cargo version doesn't match the planned one, the app must:
///   1. return `AppError::UpgradeIncorrectVersion` (existing behavior), and
///   2. fire the shutdown trigger so the host binary can flush the indexer
///      and telemetry before exiting, instead of panicking inside the ABCI
///      layer.
///
/// This test exercises (2): we attach a `watch` channel, drive the chain
/// through the halt block, and assert the trigger fired with the expected
/// halt reason.
#[tokio::test]
async fn upgrade_incorrect_version_fires_shutdown_trigger() {
    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption {
        block_time: Duration::from_seconds(1),
        ..TestOption::default()
    });

    let (halt_tx, halt_rx) = tokio::sync::watch::channel::<Option<HaltReason>>(None);
    suite.app.set_shutdown_trigger(Arc::new(halt_tx));

    // -------------------------------- Block 1 --------------------------------
    suite
        .upgrade(
            &mut accounts.owner,
            3,
            "0.1.0",
            None::<String>,
            None::<String>,
        )
        .await
        .should_succeed();

    // -------------------------------- Block 2 --------------------------------
    // Upgrade height not yet reached: trigger must stay silent.
    suite.make_empty_block().await;
    assert!(
        halt_rx.borrow().is_none(),
        "shutdown trigger fired before upgrade height",
    );

    // -------------------------------- Block 3 --------------------------------
    // Halt height reached with wrong cargo version: error returned AND
    // trigger fires with the mismatched versions.
    suite
        .try_make_empty_block()
        .await
        .should_fail_with_error(AppError::upgrade_incorrect_version(
            "0.0.0".to_string(),
            "0.1.0".to_string(),
        ));

    match halt_rx.borrow().clone() {
        Some(HaltReason::UpgradeIncorrectVersion { current, expected }) => {
            assert_eq!(current, "0.0.0");
            assert_eq!(expected, "0.1.0");
        },
        other => panic!("expected UpgradeIncorrectVersion halt reason, got {other:?}"),
    }
}
