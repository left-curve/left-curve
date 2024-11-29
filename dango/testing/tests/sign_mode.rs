use {
    dango_testing::{setup_test_naive, TestAccount, TestSuite},
    dango_types::account_factory::{self, SignMode},
    grug::{btree_set, Addr, Addressable, Coins, Defined, Hash160, ResultExt, Signer, TxOutcome},
    grug_app::NaiveProposalPreparer,
};

fn add_key<S>(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    account: &mut TestAccount<Defined<Addr>, S>,
    factory: Addr,
) -> Hash160
where
    TestAccount<Defined<Addr>, S>: Signer,
{
    let (key_hash, key) = account.add_key();

    suite
        .execute(
            account,
            factory,
            &account_factory::ExecuteMsg::RegisterKey { key_hash, key },
            Coins::default(),
        )
        .should_succeed();

    key_hash
}

fn update_sign_mode<S>(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    account: &mut TestAccount<Defined<Addr>, S>,
    factory: Addr,
    sign_mode: SignMode,
) -> TxOutcome
where
    TestAccount<Defined<Addr>, S>: Signer,
{
    suite.execute(
        account,
        factory,
        &account_factory::ExecuteMsg::ConfigureSingle { sign_mode },
        Coins::default(),
    )
}

#[test]
fn sign_with_another_key() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Add a new key
    let new_key = add_key(&mut suite, &mut accounts.owner, contracts.account_factory);

    // Set the default key
    accounts.owner.sign_mode.key_hash = new_key;

    // Sign a tx with the new key
    suite
        .transfer(
            &mut accounts.owner,
            accounts.relayer.address(),
            Coins::default(),
        )
        .should_succeed();

    // Try sign with a non valid key
    let (new_key, _) = accounts.owner.add_key();
    accounts.owner.sign_mode.key_hash = new_key;

    suite
        .transfer(
            &mut accounts.owner,
            accounts.relayer.address(),
            Coins::default(),
        )
        .should_fail_with_error("isn't associated with user `owner`");
}

#[test]
fn restricted() {
    let (mut suite, accounts, _, contracts) = setup_test_naive();

    // Take ownership of owner from accounts
    let mut owner = accounts.owner;

    // Current key
    let key_1 = owner.key_hash();

    // Add a new key
    let key_2 = add_key(&mut suite, &mut owner, contracts.account_factory);

    // Fail 1: invalid threshold
    update_sign_mode(
        &mut suite,
        &mut owner,
        contracts.account_factory,
        SignMode::Restricted {
            threshold: 3,
            allowed_keys: btree_set! {key_1, key_2},
        },
    )
    .should_fail_with_error("threshold can't be greater than total power");

    // Fail 2: key_hash not found
    update_sign_mode(
        &mut suite,
        &mut owner,
        contracts.account_factory,
        SignMode::Restricted {
            threshold: 3,
            allowed_keys: btree_set! {key_1, key_2, Hash160::from_inner([0;20])},
        },
    )
    .should_fail_with_error(
        "key 0000000000000000000000000000000000000000 doesn't exist for user owner",
    );

    // Ok update sign mode
    update_sign_mode(
        &mut suite,
        &mut owner,
        contracts.account_factory,
        SignMode::Restricted {
            threshold: 2,
            allowed_keys: btree_set! {key_1, key_2},
        },
    )
    .should_succeed();

    // Fail 3: TestAccount still be in SingleSign
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_fail_with_error("insufficient signatures: expected at least 2, got 1");

    // The sequence is not updated cause txs fails on authentication
    owner.sequence -= 1;

    // Update owner to Restricted
    let mut owner = owner.restricted(btree_set! {key_1, key_2});

    // Ok transfer
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_succeed();

    // Add a new key but don't register it
    let key_3 = owner.add_key().0;
    owner.sign_mode.key_hashes = btree_set!(key_1, key_3);

    // key_3 is not found, so it's discarded
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_fail_with_error("expected at least 2, got 1");

    owner.sequence -= 1;

    // Add a new key_properly
    owner.sign_mode.key_hashes = btree_set!(key_1, key_2);

    let key_3 = add_key(&mut suite, &mut owner, contracts.account_factory);

    update_sign_mode(
        &mut suite,
        &mut owner,
        contracts.account_factory,
        SignMode::Restricted {
            threshold: 2,
            allowed_keys: btree_set! {key_1, key_2, key_3},
        },
    )
    .should_succeed();

    // Sign with key_1 and key_2
    owner.sign_mode.key_hashes = btree_set!(key_1, key_2);
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_succeed();

    // Sign with key_1 and key_3
    owner.sign_mode.key_hashes = btree_set!(key_1, key_3);
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_succeed();

    // Sign with key_1, key_2 and key_3 (not needed but valid)
    owner.sign_mode.key_hashes = btree_set!(key_1, key_2, key_3);
    suite
        .transfer(&mut owner, accounts.relayer.address(), Coins::default())
        .should_succeed();
}
