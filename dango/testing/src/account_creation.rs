use {
    super::HyperlaneTestSuite,
    crate::{TestAccount, TestAccounts},
    dango_genesis::{Codes, Contracts},
    dango_proposal_preparer::ProposalPreparer,
    dango_types::{
        account::single::Params,
        account_factory::{self, AccountParams, Username},
        auth::Key,
        constants::usdc,
    },
    grug::{Coins, ContractWrapper, Hash256, HashExt, JsonSerExt, Op, ResultExt},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    hyperlane_types::constants::solana,
    indexer_hooked::HookedIndexer,
    pyth_client::PythClientCache,
    std::{ops::DerefMut, str::FromStr},
};

pub fn create_user_account(
    suite: &mut HyperlaneTestSuite<MemDb, RustVm, ProposalPreparer<PythClientCache>, HookedIndexer>,
    contracts: &Contracts,
    test_account: &mut TestAccount,
) {
    test_account.register_user(suite.deref_mut(), contracts.account_factory, Coins::new());
}

pub fn add_user_public_key(
    suite: &mut HyperlaneTestSuite<MemDb, RustVm, ProposalPreparer<PythClientCache>, HookedIndexer>,
    contracts: &Contracts,
    test_account: &mut TestAccount,
) -> (Key, Hash256) {
    let (_, pk) = TestAccount::new_key_pair();
    let key_hash = pk.to_json_vec().unwrap().hash256();
    suite
        .execute(
            test_account,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Insert(pk),
                key_hash,
            },
            Coins::new(),
        )
        .should_succeed();

    (pk, key_hash)
}

pub fn add_account_with_existing_user(
    suite: &mut HyperlaneTestSuite<MemDb, RustVm, ProposalPreparer<PythClientCache>, HookedIndexer>,
    contracts: &Contracts,
    test_account: &mut TestAccount,
) -> TestAccount {
    test_account
        .register_new_account(
            suite.deref_mut(),
            contracts.account_factory,
            AccountParams::Spot(Params::new(test_account.username.clone())),
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap()
}

pub fn create_user_and_account(
    suite: &mut HyperlaneTestSuite<MemDb, RustVm, ProposalPreparer<PythClientCache>, HookedIndexer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    codes: &Codes<ContractWrapper>,
    username: &str,
) -> TestAccount {
    let username = Username::from_str(username).unwrap();
    let mut user = TestAccount::new_random(username.clone()).predict_address(
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
            150_000_000,
        )
        .should_succeed();

    create_user_account(suite, contracts, &mut user);

    user
}
