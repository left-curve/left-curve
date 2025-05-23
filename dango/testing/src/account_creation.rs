use {
    super::{Factory, HyperlaneTestSuite},
    crate::{TestAccount, TestAccounts},
    dango_genesis::{Codes, Contracts},
    dango_proposal_preparer::ProposalPreparer,
    dango_types::{
        account::single::Params,
        account_factory::{
            self, AccountParams, AccountType, QueryCodeHashRequest, QueryNextAccountIndexRequest,
            RegisterUserData, Salt, Username,
        },
        constants::usdc,
    },
    grug::{Addr, Coins, ContractWrapper, Defined, HashExt, JsonSerExt, Op, QuerierExt, ResultExt},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    hyperlane_types::constants::solana,
    indexer_sql::non_blocking_indexer::NonBlockingIndexer,
    pyth_client::PythClientCache,
    std::{
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

pub fn create_user_account(
    suite: &mut HyperlaneTestSuite<
        MemDb,
        RustVm,
        ProposalPreparer<PythClientCache>,
        NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
    >,
    contracts: &Contracts,
    test_account: &mut TestAccount,
) {
    test_account.register_user(suite.deref_mut(), contracts.account_factory, Coins::new());
    // let chain_id = suite.chain_id.clone();

    // suite
    //     .execute(
    //         &mut Factory::new(contracts.account_factory),
    //         contracts.account_factory,
    //         &account_factory::ExecuteMsg::RegisterUser {
    //             seed: 0,
    //             username: test_account.username.clone(),
    //             key: test_account.first_key(),
    //             key_hash: test_account.first_key_hash(),
    //             signature: test_account
    //                 .sign_arbitrary(RegisterUserData {
    //                     username: test_account.username.clone(),
    //                     chain_id,
    //                 })
    //                 .unwrap(),
    //         },
    //         Coins::new(),
    //     )
    //     .should_succeed();

    // let index = suite
    //     .deref()
    //     .query_wasm_smart(contracts.account_factory, QueryNextAccountIndexRequest {})
    //     .unwrap();

    // let code_hash = suite
    //     .deref()
    //     .query_wasm_smart(contracts.account_factory, QueryCodeHashRequest {
    //         account_type: AccountType::Spot,
    //     })
    //     .should_succeed();

    // let address = Addr::derive(
    //     contracts.account_factory,
    //     code_hash,
    //     Salt { index }.into_bytes().as_slice(),
    // );
}

pub fn add_user_public_key(
    suite: &mut HyperlaneTestSuite<
        MemDb,
        RustVm,
        ProposalPreparer<PythClientCache>,
        NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
    >,
    contracts: &Contracts,
    mut test_account: TestAccount,
) {
    let (_, pk) = TestAccount::new_key_pair();
    let key_hash = pk.to_json_vec().unwrap().hash256();
    suite
        .execute(
            &mut test_account,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Insert(pk),
                key_hash,
            },
            Coins::new(),
        )
        .should_succeed();
}

pub fn add_account_with_existing_user(
    suite: &mut HyperlaneTestSuite<
        MemDb,
        RustVm,
        ProposalPreparer<PythClientCache>,
        NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
    >,
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
