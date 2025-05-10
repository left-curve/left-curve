use {
    crate::{
        Preset, TestAccount, TestAccounts,
        constants::{
            MOCK_CHAIN_ID, MOCK_GENESIS_TIMESTAMP, owner, user1, user2, user3, user4, user5, user6,
            user7, user8, user9,
        },
    },
    dango_genesis::{Codes, Contracts, GenesisCodes, GenesisOption, build_genesis},
    dango_proposal_preparer::ProposalPreparer,
    grug::{
        BlockInfo, ContractWrapper, Duration, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT, HashExt,
        TendermintRpcClient,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, Vm},
    grug_db_disk::{DiskDb, TempDataDir},
    grug_db_memory::MemDb,
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    indexer_httpd::context::Context,
    indexer_sql::non_blocking_indexer::NonBlockingIndexer,
    pyth_client::PythClientCache,
    std::sync::Arc,
};

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
    ID = NonBlockingIndexer<dango_indexer_sql::hooks::Hooks>,
> = grug::TestSuite<DB, VM, PP, ID>;

/// Configurable options for setting up a test.
#[derive(Default)]
pub struct TestOption {
    pub chain_id: Option<String>,
}

impl TestOption {
    pub fn with_chain_id(mut self, chain_id: &str) -> Self {
        self.chain_id = Some(chain_id.to_string());
        self
    }
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running regular tests.
pub fn setup_test() -> (TestSuite, TestAccounts, Codes<ContractWrapper>, Contracts) {
    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new_with_cache(),
        NullIndexer,
        RustVm::genesis_codes(),
        TestOption::default(),
        GenesisOption::preset_test(),
    )
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes but with a non-blocking indexer.
///
/// Used for running tests that require an indexer.
pub fn setup_test_with_indexer() -> (
    (
        TestSuiteWithIndexer,
        TestAccounts,
        Codes<ContractWrapper>,
        Contracts,
    ),
    Context,
) {
    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_hooks(dango_indexer_sql::hooks::Hooks)
        .build()
        .unwrap();

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let db = MemDb::new();
    let vm = RustVm::new();

    let (suite, accounts, codes, contracts) = setup_suite_with_db_and_vm(
        db.clone(),
        vm.clone(),
        ProposalPreparer::new_with_cache(),
        indexer,
        RustVm::genesis_codes(),
        TestOption::default(),
        GenesisOption::preset_test(),
    );

    let consensus_client = Arc::new(TendermintRpcClient::new("http://localhost:26657").unwrap());

    let httpd_context = Context::new(
        indexer_context,
        Arc::new(suite.app.clone_without_indexer()),
        consensus_client,
        indexer_path,
    );

    ((suite, accounts, codes, contracts), httpd_context)
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `NaiveProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running tests that don't require an oracle feed. For such cases, we
/// avoid adding the proposal preparer that will pull price feeds from Pyth API.
pub fn setup_test_naive() -> (
    TestSuite<NaiveProposalPreparer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
) {
    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        RustVm::genesis_codes(),
        TestOption::default(),
        GenesisOption::preset_test(),
    )
}

/// Set up a `TestSuite` with `DiskDb`, `HybridVm`, `NaiveProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running benchmarks with the hybrid VM.
pub fn setup_benchmark_hybrid(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDb, HybridVm, NullIndexer>,
    TestAccounts,
    Codes<ContractWrapper>,
    Contracts,
) {
    let codes = HybridVm::genesis_codes();
    let db = DiskDb::open(dir).unwrap();
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

/// Set up a `TestSuite` with `DiskDb`, `WasmVm`, `NaiveProposalPreparer`, and
/// `Vec<u8>` codes.
///
/// Used for running benchmarks with the Wasm VM.
pub fn setup_benchmark_wasm(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDb, WasmVm, NullIndexer>,
    TestAccounts,
    Codes<Vec<u8>>,
    Contracts,
) {
    let db = DiskDb::open(dir).unwrap();
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
)
where
    DB: Db,
    VM: Vm + GenesisCodes + Clone + 'static,
    ID: Indexer,
    PP: grug_app::ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    let (genesis_state, contracts, addresses) = build_genesis(codes.clone(), genesis_opt).unwrap();

    // TODO: here, create mock validator sets and append bridging messages to the genesis state.

    let suite = grug::TestSuite::new_with_db_vm_indexer_and_pp(
        db,
        vm,
        pp,
        indexer,
        test_opt.chain_id.unwrap_or(MOCK_CHAIN_ID.to_string()),
        Duration::from_millis(250),
        1_000_000,
        BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: MOCK_GENESIS_TIMESTAMP,
        },
        genesis_state,
    );

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

    let accounts = TestAccounts {
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
    };

    (suite, accounts, codes, contracts)
}
