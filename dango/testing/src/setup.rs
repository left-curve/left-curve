use {
    crate::{
        Preset, TestAccount, TestAccounts,
        constants::{owner, user1, user2, user3, user4, user5, user6, user7, user8, user9},
    },
    dango_genesis::{Codes, Contracts, GenesisCodes, GenesisOption, build_genesis},
    dango_proposal_preparer::ProposalPreparer,
    dango_types::{
        gateway::{Domain, Remote},
        warp,
    },
    grug::{
        Addr, BlockInfo, Coins, ContractWrapper, Duration, Message, TendermintRpcClient, Uint128,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, SimpleCommitment, Vm},
    grug_db_disk::DiskDb,
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    hyperlane_testing::MockValidatorSets,
    hyperlane_types::{Addr32, mailbox},
    indexer_hooked::HookedIndexer,
    pyth_client::PythClientCache,
    std::sync::Arc,
    temp_rocksdb::TempDataDir,
};

/// Configurable options for setting up a test.
pub struct TestOption {
    pub chain_id: String,
    pub block_time: Duration,
    pub default_gas_limit: u64,
    pub genesis_block: BlockInfo,
    /// A function that takes a list of test accounts that will be created, and
    /// returns a list of incoming bridge transfers to be appended to the
    /// genesis state.
    pub bridge_ops: fn(&TestAccounts) -> Vec<BridgeOp>,
    pub mocked_clickhouse: bool,
}

impl TestOption {
    pub fn with_mocked_clickhouse(self) -> Self {
        Self {
            mocked_clickhouse: true,
            ..Self::default()
        }
    }
}

impl Default for TestOption {
    fn default() -> Self {
        Preset::preset_test()
    }
}

/// A bridge operation to be included in the genesis state.
pub struct BridgeOp {
    pub remote: Remote,
    pub amount: Uint128,
    pub recipient: Addr,
}

pub type TestSuite<
    PP = ProposalPreparer<PythClientCache>,
    DB = MemDb,
    VM = RustVm,
    ID = NullIndexer,
> = grug::TestSuite<DB, VM, PP, ID>;

pub type TestSuiteWithIndexer<
    PP = ProposalPreparer<PythClientCache>,
    DB = MemDb,
    VM = RustVm,
    ID = HookedIndexer,
> = grug::TestSuite<DB, VM, PP, ID>;

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer` with cached
/// Pyth Lazer client, and `ContractWrapper` codes.
///
/// Used for running regular tests.
pub fn setup_test(
    test_opt: TestOption,
) -> (
    TestSuite,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new_with_cache(),
        NullIndexer,
        RustVm::genesis_codes(),
        test_opt,
        GenesisOption::preset_test(),
    )
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `NaiveProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running tests that don't require an oracle feed. For such cases, we
/// avoid adding the proposal preparer that will pull price feeds from Pyth API.
pub fn setup_test_naive(
    test_opt: TestOption,
) -> (
    TestSuite<NaiveProposalPreparer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    setup_test_naive_with_custom_genesis(test_opt, GenesisOption::preset_test())
}

pub fn setup_test_naive_with_custom_genesis(
    test_opt: TestOption,
    genesis_opt: GenesisOption,
) -> (
    TestSuite<NaiveProposalPreparer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    )
}

pub async fn setup_test_with_indexer(
    test_opt: TestOption,
) -> (
    TestSuiteWithIndexer,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
    indexer_httpd::context::Context,
    dango_httpd::context::Context,
    dango_indexer_clickhouse::context::Context,
) {
    setup_test_with_indexer_and_custom_genesis(test_opt, GenesisOption::preset_test()).await
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes but with a non-blocking indexer.
///
/// Used for running tests that require an indexer.
/// Synchronous wrapper for setup_test_with_indexer_async
pub async fn setup_test_with_indexer_and_custom_genesis(
    options: TestOption,
    genesis_opt: GenesisOption,
) -> (
    TestSuiteWithIndexer,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
    indexer_httpd::context::Context,
    dango_httpd::context::Context,
    dango_indexer_clickhouse::context::Context,
) {
    let indexer = indexer_sql::IndexerBuilder::default()
        // We'll use this with a random database.
        // .with_database_url("postgres://postgres@localhost/grug_test")
        .with_memory_database()
        .with_database_max_connections(1)
        .build()
        .unwrap();

    let indexer_context = indexer.context.clone();

    let mut hooked_indexer = HookedIndexer::new();

    let indexer_cache = indexer_cache::Cache::new_with_tempdir();
    let indexer_cache_context = indexer_cache.context.clone();

    // Create a separate context for dango indexer (shares DB but has independent pubsub)
    let dango_context: dango_indexer_sql::context::Context = indexer
        .context
        .with_separate_pubsub()
        .await
        .expect("Failed to create separate context for dango indexer in test setup")
        .into();

    let dango_indexer = dango_indexer_sql::indexer::Indexer::new(
        indexer_sql::indexer::RuntimeHandler::from_handle(tokio::runtime::Handle::current()),
        dango_context.clone(),
    );

    let mut clickhouse_context = dango_indexer_clickhouse::context::Context::new(
        format!(
            "http://{}:{}",
            std::env::var("CLICKHOUSE_HOST").unwrap_or("localhost".to_string()),
            std::env::var("CLICKHOUSE_PORT").unwrap_or("8123".to_string())
        ),
        std::env::var("CLICKHOUSE_DATABASE").unwrap_or("grug_dev".to_string()),
        std::env::var("CLICKHOUSE_USER").unwrap_or("default".to_string()),
        std::env::var("CLICKHOUSE_PASSWORD").unwrap_or("".to_string()),
    );

    if !options.mocked_clickhouse {
        clickhouse_context = clickhouse_context.with_test_database().await.unwrap();
    } else {
        clickhouse_context = clickhouse_context.with_mock();
    }

    hooked_indexer.add_indexer(indexer_cache).unwrap();
    hooked_indexer.add_indexer(indexer).unwrap();
    hooked_indexer.add_indexer(dango_indexer).unwrap();

    let clickhouse_indexer = dango_indexer_clickhouse::indexer::Indexer::new(
        indexer_sql::indexer::RuntimeHandler::from_handle(tokio::runtime::Handle::current()),
        clickhouse_context.clone(),
    );
    hooked_indexer.add_indexer(clickhouse_indexer).unwrap();

    let db = MemDb::new();
    let vm = RustVm::new();

    let (suite, accounts, codes, contracts, validator_sets) = setup_suite_with_db_and_vm(
        db.clone(),
        vm.clone(),
        ProposalPreparer::new_with_cache(),
        hooked_indexer,
        RustVm::genesis_codes(),
        options,
        genesis_opt,
    );

    clickhouse_context.start_cache().await.unwrap();

    let consensus_client = Arc::new(TendermintRpcClient::new("http://localhost:26657").unwrap());

    let indexer_httpd_context = indexer_httpd::context::Context::new(
        indexer_cache_context,
        indexer_context,
        Arc::new(suite.app.clone_without_indexer()),
        consensus_client,
    );

    let dango_httpd_context = dango_httpd::context::Context::new(
        indexer_httpd_context.clone(),
        clickhouse_context.clone(),
        dango_context,
        None,
    );

    (
        suite,
        accounts,
        codes,
        contracts,
        validator_sets,
        indexer_httpd_context,
        dango_httpd_context,
        clickhouse_context,
    )
}

/// Set up a `TestSuite` with `DiskDbLite`, `RustVm`, `NaiveProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running benchmarks with the Rust VM.
pub fn setup_benchmark_rust(
    dir: &TempDataDir,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDb<SimpleCommitment>, RustVm, NullIndexer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    let db = DiskDb::open(dir).unwrap();
    let codes = RustVm::genesis_codes();
    let vm = RustVm::new();

    setup_suite_with_db_and_vm(
        db,
        vm,
        NaiveProposalPreparer,
        NullIndexer,
        codes,
        TestOption::default(),
        GenesisOption::preset_test(),
    )
}

pub fn setup_suite_with_db_and_vm<DB, VM, PP, ID>(
    db: DB,
    vm: VM,
    pp: PP,
    indexer: ID,
    codes: Codes<VM::Code>,
    test_opt: TestOption,
    genesis_opt: GenesisOption,
) -> (
    TestSuite<PP, DB, VM, ID>,
    TestAccounts,
    Codes<VM::Code>,
    Contracts,
    MockValidatorSets,
)
where
    DB: Db,
    VM: Vm + GenesisCodes + Clone + Send + Sync + 'static,
    ID: Indexer,
    PP: grug_app::ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    let local_domain = genesis_opt.hyperlane.local_domain;

    // Build the genesis state.
    let (mut genesis_state, contracts, addresses) =
        build_genesis(codes.clone(), genesis_opt).unwrap();

    // Create the test accounts.
    let accounts = {
        let owner = TestAccount::new_from_private_key(owner::PRIVATE_KEY)
            .set_user_index(0)
            .set_address(addresses[0]);
        let user1 = TestAccount::new_from_private_key(user1::PRIVATE_KEY)
            .set_user_index(1)
            .set_address(addresses[1]);
        let user2 = TestAccount::new_from_private_key(user2::PRIVATE_KEY)
            .set_user_index(2)
            .set_address(addresses[2]);
        let user3 = TestAccount::new_from_private_key(user3::PRIVATE_KEY)
            .set_user_index(3)
            .set_address(addresses[3]);
        let user4 = TestAccount::new_from_private_key(user4::PRIVATE_KEY)
            .set_user_index(4)
            .set_address(addresses[4]);
        let user5 = TestAccount::new_from_private_key(user5::PRIVATE_KEY)
            .set_user_index(5)
            .set_address(addresses[5]);
        let user6 = TestAccount::new_from_private_key(user6::PRIVATE_KEY)
            .set_user_index(6)
            .set_address(addresses[6]);
        let user7 = TestAccount::new_from_private_key(user7::PRIVATE_KEY)
            .set_user_index(7)
            .set_address(addresses[7]);
        let user8 = TestAccount::new_from_private_key(user8::PRIVATE_KEY)
            .set_user_index(8)
            .set_address(addresses[8]);
        let user9 = TestAccount::new_from_private_key(user9::PRIVATE_KEY)
            .set_user_index(9)
            .set_address(addresses[9]);

        TestAccounts {
            owner,
            user1,
            user2,
            user3,
            user4,
            user5,
            user6,
            user7,
            user8,
            user9,
        }
    };

    // Create the mock validator sets.
    // TODO: For now, we always use the preset mock. It may not match the ones
    // in the genesis state. We should generate this based on the `genesis_opt`.
    let validator_sets = MockValidatorSets::new_preset(false);

    for op in (test_opt.bridge_ops)(&accounts) {
        match op.remote {
            Remote::Warp { domain, contract } => {
                genesis_state.msgs.push(build_genesis_warp_msg(
                    &contracts,
                    &validator_sets,
                    domain,
                    local_domain,
                    contract,
                    op.amount,
                    op.recipient,
                ));
            },
            Remote::Bitcoin => {
                todo!("bitcoin bridge isn't supported yet");
            },
        }
    }

    let suite = grug::TestSuite::new_with_db_vm_indexer_and_pp(
        db,
        vm,
        pp,
        indexer,
        None, // TODO: support customizing upgrade handler in tests
        test_opt.chain_id,
        test_opt.block_time,
        test_opt.default_gas_limit,
        test_opt.genesis_block,
        genesis_state,
    );

    (suite, accounts, codes, contracts, validator_sets)
}

fn build_genesis_warp_msg(
    contracts: &Contracts,
    validator_sets: &MockValidatorSets,
    origin_domain: Domain,
    destination_domain: Domain,
    sender: Addr32,
    amount: Uint128,
    recipient: Addr,
) -> Message {
    let validator_set = validator_sets.get(origin_domain);

    let warp_msg = warp::TokenMessage {
        recipient: recipient.into(),
        amount,
        metadata: Default::default(), // Metadata isn't supported yet.
    };

    let (_, raw_message, raw_metadata) = validator_set
        .unwrap_or_else(|| panic!("no mock validator set found for domain `{origin_domain}`"))
        .sign(
            sender,
            destination_domain,
            contracts.warp,
            warp_msg.encode(),
        );

    Message::execute(
        contracts.hyperlane.mailbox,
        &mailbox::ExecuteMsg::Process {
            raw_message,
            raw_metadata,
        },
        Coins::new(),
    )
    .unwrap()
}
