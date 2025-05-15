use {
    assertor::*,
    dango_testing::{Factory, HyperlaneTestSuite, TestAccount, setup_test_with_indexer},
    dango_types::{
        account::single,
        account_factory::{self, Account, AccountParams, RegisterUserData, Username},
        bank,
        constants::usdc,
    },
    grug::{
        Addressable, Coins, HashExt, Inner, QuerierExt, ResultExt, Uint128, btree_map, coins,
        setup_tracing_subscriber,
    },
    hyperlane_types::constants::solana,
    sea_orm::EntityTrait,
    std::str::FromStr,
};

#[test]
fn index_account_creations() {
    setup_tracing_subscriber(tracing::Level::INFO);

    let (suite, mut accounts, codes, contracts, validator_sets, _) = setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let chain_id = suite.chain_id.clone();

    // tracing::debug_span!(
    //     "Will use indexerpath for testing: {:?}",
    //     context.indexer_path
    // );

    // Copied from `user_onboarding`` test
    // Create a new key offchain; then, predict what its address would be.
    let username = Username::from_str("user").unwrap();
    let user = TestAccount::new_random(username.clone()).predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &user,
            10_000_000,
        )
        .should_succeed();

    // The transfer should be an orphaned transfer. The bank contract should be
    // holding the 10 USDC.
    suite
        .query_balance(&contracts.bank, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // The orphaned transfer should have been recorded.
    suite
        .query_wasm_smart(contracts.bank, bank::QueryOrphanedTransferRequest {
            sender: contracts.gateway,
            recipient: user.address(),
        })
        .should_succeed_and_equal(coins! { usdc::DENOM.clone() => 10_000_000 });

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                seed: 0,
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // The user's account info should have been recorded in account factory.
    // Note: a user's first ever account is always a spot account.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.address() => Account {
                // We have 10 genesis accounts (owner + users 1-9), indexed from
                // zero, so this one should have the index of 10.
                index: 10,
                params: AccountParams::Spot(single::Params::new(user.username.clone() )),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // The account creation should have been indexed.
    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let users = dango_indexer_sql::entity::users::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let accounts = dango_indexer_sql::entity::accounts::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let account_users = dango_indexer_sql::entity::accounts_users::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let public_keys = dango_indexer_sql::entity::public_keys::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(
                users
                    .iter()
                    .map(|t| t.username.as_str())
                    .collect::<Vec<_>>()
            )
            .is_equal_to(vec![username.as_ref()]);

            assert_that!(users).has_length(1);
            assert_that!(accounts).has_length(1);
            assert_that!(account_users).has_length(1);
            assert_that!(public_keys).has_length(1);

            let public_key = public_keys.first().unwrap();

            assert_that!(&public_key.username).is_equal_to(username.inner());
            assert_that!(public_key.key_hash).is_equal_to(user.first_key_hash().to_string());
            assert_that!(public_key.public_key).is_equal_to(user.first_key().to_string());

            dbg!(users);
            dbg!(public_keys);
            dbg!(accounts);
            dbg!(account_users);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch accounts");
}

#[ignore]
#[test]
fn index_previous_blocks() {
    setup_tracing_subscriber(tracing::Level::INFO);

    let (suite, mut accounts, codes, contracts, validator_sets, _) = setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let chain_id = suite.chain_id.clone();
    let username = "user";

    // tracing::debug_span!(
    //     "Will use indexerpath for testing: {:?}",
    //     context.indexer_path
    // );

    // Copied from `user_onboarding`` test
    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random(Username::from_str(username).unwrap()).predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &user,
            10_000_000,
        )
        .should_succeed();

    // The transfer should be an orphaned transfer. The bank contract should be
    // holding the 10 USDC.
    suite
        .query_balance(&contracts.bank, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // The orphaned transfer should have been recorded.
    suite
        .query_wasm_smart(contracts.bank, bank::QueryOrphanedTransferRequest {
            sender: contracts.warp,
            recipient: user.address(),
        })
        .should_succeed_and_equal(coins! { usdc::DENOM.clone() => 10_000_000 });

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                seed: 0,
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // The user's account info should have been recorded in account factory.
    // Note: a user's first ever account is always a spot account.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.address() => Account {
                // We have 10 genesis accounts (owner + users 1-9), indexed from
                // zero, so this one should have the index of 10.
                index: 10,
                params: AccountParams::Spot(single::Params::new(user.username.clone() )),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // The transfer should have been indexed.
    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let accounts = dango_indexer_sql::entity::accounts::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(accounts).has_length(1);

            // assert_that!(
            //     accounts
            //         .iter()
            //         .map(|t| t.username.as_str())
            //         .collect::<Vec<_>>()
            // )
            // .is_equal_to(vec![username]);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch accounts");
}
