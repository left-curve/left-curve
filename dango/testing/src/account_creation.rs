use {
    super::{Factory, HyperlaneTestSuite},
    crate::{TestAccount, TestAccounts},
    dango_genesis::{Codes, Contracts},
    dango_proposal_preparer::ProposalPreparer,
    dango_types::{
        account_factory::{self, RegisterUserData, Username},
        auth::Key,
    },
    grug::{Coins, ContractWrapper, Hash256, HashExt, ResultExt},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    hyperlane_types::constants::solana,
    indexer_sql::non_blocking_indexer::NonBlockingIndexer,
    pyth_client::PythClientCache,
    std::str::FromStr,
};

pub fn create_user_account_with_key(
    suite: &mut HyperlaneTestSuite<
        MemDb,
        RustVm,
        ProposalPreparer<PythClientCache>,
        NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
    >,
    contracts: &Contracts,
    user: &TestAccount,
    chain_id: String,
    public_key: Key,
    public_key_hash: Hash256,
) {
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
                key: public_key,
                key_hash: public_key_hash,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id,
                    })
                    .unwrap(),
            },
            Coins::new(),
        )
        .should_succeed();
}

pub fn create_user_and_accounts(
    suite: &mut HyperlaneTestSuite<
        MemDb,
        RustVm,
        ProposalPreparer<PythClientCache>,
        NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
    >,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    codes: &Codes<ContractWrapper>,
    username: &str,
) -> TestAccount {
    let username = Username::from_str(username).unwrap();
    let chain_id = suite.chain_id.clone();
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

    create_user_account_with_key(
        suite,
        contracts,
        &user,
        chain_id,
        user.first_key(),
        user.first_key_hash(),
    );

    user
}
