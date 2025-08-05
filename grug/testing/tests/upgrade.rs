use {
    grug_app::{CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    grug_math::{Bytable, NextNumber, Uint128, Uint256},
    grug_testing::TestBuilder,
    grug_types::{
        BorshSerExt, Coins, Denom, Duration, Order, QuerierExt, ResultExt, StdResult, Timestamp,
        coins,
    },
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
        .set_upgrade_handler(3, |mut storage, _vm| {
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
/// In practice, this is easier done with the contract migration, but for test
/// purpose this is fine.
#[test]
fn upgrading_with_calling_contract() {
    mod new_mock_bank {
        use {
            grug_math::Uint256,
            grug_storage::Map,
            grug_types::{Addr, Denom},
        };

        pub const BALANCES_BY_ADDR: Map<(Addr, &Denom), Uint256> = Map::new("bu");
    }

    let denom = Denom::from_str("oonga").unwrap();

    let (mut suite, accounts) = TestBuilder::new()
        .add_account("owner", coins! { denom.clone() => 123 })
        .set_owner("owner")
        .set_upgrade_handler(3, |storage, _vm| {
            // Get the prefixed storage of the bank contract.
            let mut bank_storage = {
                let cfg = CONFIG.load(storage.as_ref())?;
                StorageProvider::new(storage, &[CONTRACT_NAMESPACE, cfg.bank.as_ref()])
            };

            // Loop through all the balances and convert them to 256-bit.
            for ((addr, denom), amount) in grug_mock_bank::BALANCES_BY_ADDR
                .range(&bank_storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?
            {
                let amount = amount.into_next();
                new_mock_bank::BALANCES_BY_ADDR.save(&mut bank_storage, (addr, &denom), &amount)?;
            }

            Ok(())
        })
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
