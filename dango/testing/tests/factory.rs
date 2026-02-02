use {
    dango_account_factory::{ACCOUNT_COUNT_BY_USER, MAX_ACCOUNTS_PER_USER},
    dango_genesis::{AccountOption, GenesisOption},
    dango_testing::{
        Factory, HyperlaneTestSuite, Preset, TestAccount, setup_test_naive,
        setup_test_naive_with_custom_genesis,
    },
    dango_types::{
        account,
        account_factory::{self, Account, RegisterUserData, UserIndexOrName},
        auth::AccountStatus,
        bank,
        constants::usdc,
    },
    grug::{
        Addressable, Coins, HashExt, JsonSerExt, Message, NonEmpty, Op, QuerierExt, ResultExt,
        Signer, StorageQuerier, Uint128, btree_map, coins,
    },
    hyperlane_types::constants::solana,
};

/// Prior to PR [#1460](https://github.com/left-curve/left-curve/pull/1460),
/// users are expected to first make a deposit before sending the `RegisterUser`
/// message. Sending the `RegisterUser` message without a deposit resulting in
/// the transaction failing. This design has drawbacks; see the PR's description.
/// Since PR #1460, this test now reflects the intended onboarding procedure.
#[test]
fn onboarding_without_deposit() {
    let (suite, mut accounts, codes, contracts, validator_sets) =
        setup_test_naive_with_custom_genesis(Default::default(), GenesisOption {
            account: AccountOption {
                minimum_deposit: coins! { usdc::DENOM.clone() => 10_000_000 },
                ..Preset::preset_test()
            },
            ..Preset::preset_test()
        });
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    // Make an empty block to advance block height from 0 to 1.
    //
    // The reason of this is when the chain does `CheckTx`, it does it under the
    // state of the _last finalized block_. Without advancing the block here,
    // that would be block 0, in other words the genesis block. The single-signature
    // account won't claim orphaned transfers during genesis. For a realistic test,
    // we do `CheckTx` at a post-genesis block.
    suite.make_empty_block();

    let chain_id = suite.chain_id.clone();

    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account.to_bytes().hash256(),
        true,
    );

    // Send the register user transaction without making a deposit first.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 3,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The account should have been created in the `Inactive` state.
    suite
        .query_wasm_smart(user.address(), account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Inactive);

    // Attempting to send a transaction at this time. `CheckTx` should fail.
    let mut user = user.query_user_index(suite.querier());
    let tx = user
        .sign_transaction(
            NonEmpty::new_unchecked(vec![
                Message::transfer(user.address(), Coins::new()).unwrap(),
            ]),
            &suite.chain_id,
            100_000,
        )
        .unwrap();

    suite
        .check_tx(tx.clone())
        .should_fail_with_error(format!("account {} is not active", user.address()));

    // Make a deposit of the minimum amount.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &user,
            10_000_000, // Minimum deposit is 10_000_000. Need to send at this that amount.
        )
        .should_succeed();

    // Account should have been activated.
    suite
        .query_wasm_smart(user.address(), account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Active);

    // Try again, should succeed.
    suite.check_tx(tx).should_succeed();

    // User opens a new account. While his first account required an initial
    // deposit, any subsequent account should be activated by default.
    let user_index = user.user_index();
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {},
            Coins::new(),
        )
        .should_succeed();

    // Ensure the user now has two accounts and they are both active.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                user: UserIndexOrName::Index(user_index),
            },
        )
        .should_succeed_and(|accounts| {
            accounts.len() == 2
                && accounts.iter().all(|(address, _)| {
                    suite
                        .query_wasm_path(*address, dango_auth::account::STATUS.path())
                        .unwrap()
                        .is_active()
                })
        });
}

/// If minimum deposit is zero, then the account is automatically activated.
/// No need to make a deposit.
#[test]
fn onboarding_without_deposit_when_minimum_deposit_is_zero() {
    // Set up the test with minimum deposit set to zero.
    let (mut suite, mut accounts, codes, contracts, _) = setup_test_naive(Default::default());

    let chain_id = suite.chain_id.clone();

    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account.to_bytes().hash256(),
        true,
    );

    // Attempt to register a user without making a deposit.
    suite
        .execute(
            &mut accounts.owner,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 3,
                signature: user.sign_arbitrary(RegisterUserData { chain_id }).unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Now that the user has been created, query it's index.
    let user = user.query_user_index(suite.querier());

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                user: UserIndexOrName::Index(user.user_index()),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // The user's account info should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                user: UserIndexOrName::Index(user.user_index()),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.address() => Account {
                // We have 10 genesis accounts (owner + users 1-9), indexed from
                // zero, so this one should have the index of 10.
                index: 10,
                owner: user.user_index(),
            },
        });

    // The newly created account should be active.
    suite
        .query_wasm_smart(user.address(), account::QueryStatusRequest {})
        .should_succeed_and_equal(AccountStatus::Active);

    // The newly created account should have zero balance.
    suite
        .query_balances(&user)
        .should_succeed_and(|coins| coins.is_empty());
}

/// Since PR [#1460](https://github.com/left-curve/left-curve/pull/1460), it's
/// not longer necessary to make a deposit before onboarding.
/// However, we keep this test for the edge case -- what if someone sends a
/// transfer before creating the account? The user needs to be able to recover
/// the funds.
#[test]
fn onboarding_with_deposit_when_minimum_deposit_is_zero() {
    // Set up the test with minimum deposit set to zero.
    let (suite, mut accounts, codes, contracts, validator_sets) =
        setup_test_naive(Default::default());
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    // Generate a random key for the user.
    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit, even though not required.
    // The deposit should be held in the bank contract as an orphaned transfer.
    suite
        .receive_warp_transfer(
            &mut accounts.owner,
            solana::DOMAIN,
            solana::USDC_WARP,
            &user,
            10_000_000,
        )
        .should_succeed();

    // Sign the `RegisterUserData`.
    let signature = user
        .sign_arbitrary(RegisterUserData {
            chain_id: suite.chain_id.clone(),
        })
        .unwrap();

    // Onboard the user.
    // Attempt to register a user without making a deposit.
    suite
        .execute(
            &mut accounts.owner,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 3,
                signature,
            },
            Coins::new(),
        )
        .should_succeed();

    // Now that the user has been created, he can claim the orphaned transfer.
    let mut user = user.query_user_index(suite.querier());
    let user_address = user.address();

    suite
        .execute(
            &mut user,
            contracts.bank,
            &bank::ExecuteMsg::RecoverTransfer {
                sender: contracts.gateway,
                recipient: user_address,
            },
            Coins::new(),
        )
        .should_succeed();

    // Make sure a single-signature account is created with the deposited balance.
    suite
        .query_balance(&user, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));
}

#[test]
fn update_key() {
    let (mut suite, _, codes, contracts, _) = setup_test_naive(Default::default());

    let chain_id = suite.chain_id.clone();

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        0,
        codes.account.to_bytes().hash256(),
        true,
    );

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in orphaned transfer in bank.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 0,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Now that the user has been created, query it's index.
    let mut user = user.query_user_index(suite.querier());

    // Try to delete last key, should fail.
    let first_key_hash = user.first_key_hash();
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Delete,
                key_hash: first_key_hash,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "can't delete the last key associated with user index {}",
            user.user_index()
        ));

    // Query keys should return only one key.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                user: UserIndexOrName::Index(user.user_index()),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // Add a new key to the user's account.
    let (_, pk) = TestAccount::new_key_pair();
    let key_hash = pk.to_json_vec().unwrap().hash256();
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Insert(pk),
                key_hash,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query keys should return two keys.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                user: UserIndexOrName::Index(user.user_index()),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.first_key_hash() => user.first_key(),
            key_hash => pk,
        });

    // It shouldn't be able to add the same key more than once.
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Insert(pk),
                key_hash,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "key is already associated with user index {}",
            user.user_index()
        ));

    // Delete the first key should be possible since there is another key.
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Delete,
                key_hash: first_key_hash,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query keys should return only one key.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                user: UserIndexOrName::Index(user.user_index()),
            },
        )
        .should_succeed_and_equal(btree_map! { key_hash => pk });
}

#[test]
fn single_signature_account_count_limit() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let user_index = accounts.user1.user_index();

    // User 1 should have one account now. Open 4 more.
    for _ in 2..=MAX_ACCOUNTS_PER_USER {
        suite
            .execute(
                &mut accounts.user1,
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterAccount {},
                Coins::new(),
            )
            .should_succeed();
    }

    // Query user 1's account count that is stored in factory. Should be 5.
    suite
        .query_wasm_path(
            contracts.account_factory,
            &ACCOUNT_COUNT_BY_USER.path(user_index),
        )
        .should_succeed_and_equal(MAX_ACCOUNTS_PER_USER);

    // Attempt to open one more account. Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {},
            Coins::new(),
        )
        .should_fail_with_error(format!("user {user_index} has reached max account count"));
}
