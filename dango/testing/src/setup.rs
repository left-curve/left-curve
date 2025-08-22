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
        Addr, BlockInfo, Coins, ContractWrapper, Duration, HashExt, Message, TendermintRpcClient,
        Uint128,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, Vm},
    grug_db_disk_lite::DiskDbLite,
    grug_db_memory::MemDb,
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    hyperlane_testing::MockValidatorSets,
    hyperlane_types::{Addr32, mailbox},
    indexer_hooked::HookedIndexer,
    pyth_client::PythClientCache,
    pyth_lazer::PythClientLazer,
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

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes.
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

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running regular tests.
pub fn setup_test_lazer(
    test_opt: TestOption,
) -> (
    TestSuite<ProposalPreparer<PythClientLazer>>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new_with_lazer(),
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

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes but with a non-blocking indexer.
///
/// Used for running tests that require an indexer.
/// Synchronous wrapper for setup_test_with_indexer_async
pub async fn setup_test_with_indexer(
    options: TestOption,
) -> (
    TestSuiteWithIndexer,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
    indexer_httpd::context::Context,
    dango_httpd::context::Context,
    indexer_clickhouse::context::Context,
) {
    let indexer = indexer_sql::IndexerBuilder::default()
        .with_memory_database()
        .with_database_max_connections(1)
        .build()
        .unwrap();

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    // Create a shared runtime handler that uses the same tokio runtime
    let shared_runtime_handle =
        indexer_sql::indexer::RuntimeHandler::from_handle(indexer.handle.handle().clone());
    let shared_runtime_handle2 =
        indexer_sql::indexer::RuntimeHandler::from_handle(indexer.handle.handle().clone());

    let mut hooked_indexer = HookedIndexer::new();

    // Create a separate context for dango indexer (shares DB but has independent pubsub)
    let dango_context: dango_indexer_sql::context::Context = indexer
        .context
        .with_separate_pubsub()
        .await
        .expect("Failed to create separate context for dango indexer in test setup")
        .into();

    let dango_indexer =
        dango_indexer_sql::indexer::Indexer::new(shared_runtime_handle, dango_context.clone());

    let mut clickhouse_context = indexer_clickhouse::context::Context::new(
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

    hooked_indexer.add_indexer(indexer).unwrap();
    hooked_indexer.add_indexer(dango_indexer).unwrap();

    let clickhouse_indexer = indexer_clickhouse::indexer::Indexer::new(
        shared_runtime_handle2,
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
        GenesisOption::preset_test(),
    );

    clickhouse_context.start_candle_cache().await.unwrap();

    let consensus_client = Arc::new(TendermintRpcClient::new("http://localhost:26657").unwrap());

    let indexer_httpd_context = indexer_httpd::context::Context::new(
        indexer_context,
        Arc::new(suite.app.clone_without_indexer()),
        consensus_client,
        indexer_path,
    );

    let dango_httpd_context = dango_httpd::context::Context::new(
        indexer_httpd_context.clone(),
        clickhouse_context.clone(),
        dango_context,
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

/// Set up a `TestSuite` with `DiskDbLite`, `HybridVm`, `NaiveProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running benchmarks with the hybrid VM.
pub fn setup_benchmark_hybrid(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDbLite, HybridVm, NullIndexer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
    MockValidatorSets,
) {
    let db = DiskDbLite::open(dir).unwrap();
    let codes = HybridVm::genesis_codes();
    let vm = HybridVm::new(wasm_cache_size, [
        codes.account_factory.to_bytes().hash256(),
        codes.account_margin.to_bytes().hash256(),
        codes.account_multi.to_bytes().hash256(),
        codes.account_spot.to_bytes().hash256(),
        codes.bank.to_bytes().hash256(),
        codes.dex.to_bytes().hash256(),
        codes.gateway.to_bytes().hash256(),
        codes.hyperlane.ism.to_bytes().hash256(),
        codes.hyperlane.mailbox.to_bytes().hash256(),
        codes.hyperlane.va.to_bytes().hash256(),
        codes.lending.to_bytes().hash256(),
        codes.oracle.to_bytes().hash256(),
        codes.taxman.to_bytes().hash256(),
        codes.vesting.to_bytes().hash256(),
        codes.warp.to_bytes().hash256(),
    ]);

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

/// Set up a `TestSuite` with `DiskDbLite`, `WasmVm`, `NaiveProposalPreparer`, and
/// `Vec<u8>` codes.
///
/// Used for running benchmarks with the Wasm VM.
pub fn setup_benchmark_wasm(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDbLite, WasmVm, NullIndexer>,
    TestAccounts,
    Codes<Vec<u8>>,
    Contracts,
    MockValidatorSets,
) {
    let db = DiskDbLite::open(dir).unwrap();
    let vm = WasmVm::new(wasm_cache_size);

    setup_suite_with_db_and_vm(
        db,
        vm,
        NaiveProposalPreparer,
        NullIndexer,
        WasmVm::genesis_codes(),
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
        let owner = TestAccount::new_from_private_key(owner::USERNAME.clone(), owner::PRIVATE_KEY);
        let user1 = TestAccount::new_from_private_key(user1::USERNAME.clone(), user1::PRIVATE_KEY);
        let user2 = TestAccount::new_from_private_key(user2::USERNAME.clone(), user2::PRIVATE_KEY);
        let user3 = TestAccount::new_from_private_key(user3::USERNAME.clone(), user3::PRIVATE_KEY);
        let user4 = TestAccount::new_from_private_key(user4::USERNAME.clone(), user4::PRIVATE_KEY);
        let user5 = TestAccount::new_from_private_key(user5::USERNAME.clone(), user5::PRIVATE_KEY);
        let user6 = TestAccount::new_from_private_key(user6::USERNAME.clone(), user6::PRIVATE_KEY);
        let user7 = TestAccount::new_from_private_key(user7::USERNAME.clone(), user7::PRIVATE_KEY);
        let user8 = TestAccount::new_from_private_key(user8::USERNAME.clone(), user8::PRIVATE_KEY);
        let user9 = TestAccount::new_from_private_key(user9::USERNAME.clone(), user9::PRIVATE_KEY);

        TestAccounts {
            owner: owner.set_address(&addresses),
            user1: user1.set_address(&addresses),
            user2: user2.set_address(&addresses),
            user3: user3.set_address(&addresses),
            user4: user4.set_address(&addresses),
            user5: user5.set_address(&addresses),
            user6: user6.set_address(&addresses),
            user7: user7.set_address(&addresses),
            user8: user8.set_address(&addresses),
            user9: user9.set_address(&addresses),
        }
    };

    // Create the mock validator sets.
    // TODO: For now, we always use the preset mock. It may not match the ones
    // in the genesis state. We should generate this based on the `genesis_opt`.
    let validator_sets = MockValidatorSets::new_preset();

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
