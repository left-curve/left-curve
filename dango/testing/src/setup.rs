use {
    crate::{Accounts, TestAccount},
    dango_app::ProposalPreparer,
    dango_genesis::{build_genesis, read_wasm_files, Codes, Contracts, GenesisUser},
    grug::{
        btree_map, Binary, BlockInfo, Coin, ContractBuilder, ContractWrapper, Duration,
        NumberConst, Timestamp, Udec128, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, Vm},
    grug_db_disk::{DiskDb, TempDataDir},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    std::{env, fmt::Display, path::PathBuf, sync::LazyLock},
};

pub const CHAIN_ID: &str = "dev-1";

/// The chain's genesis timestamp.
pub const GENESIS_TIMESTAMP: Timestamp = Timestamp::from_days(365);

pub static TOKEN_FACTORY_CREATION_FEE: LazyLock<Coin> =
    LazyLock::new(|| Coin::new("uusdc", 10_000_000).unwrap());

pub type TestSuite<PP = ProposalPreparer, DB = MemDb, VM = RustVm, ID = NullIndexer> =
    grug::TestSuite<DB, VM, PP, ID>;

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer` and `ContractWrapper` codes.
pub fn setup_test() -> (TestSuite, Accounts, Codes<ContractWrapper>, Contracts) {
    let codes = build_codes();

    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        codes,
        ProposalPreparer::new(),
        NullIndexer,
    )
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `NaiveProposalPreparer` and `ContractWrapper` codes.
pub fn setup_test_naive() -> (
    TestSuite<NaiveProposalPreparer>,
    Accounts,
    Codes<ContractWrapper>,
    Contracts,
) {
    let codes = build_codes();

    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        codes,
        NaiveProposalPreparer,
        NullIndexer,
    )
}

/// Set up a `TestSuite` with `DiskDb`, `WasmVm`, and `Vec<u8>` codes.
/// Used for benchmarks.
pub fn setup_benchmark(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDb, WasmVm, NullIndexer>,
    Accounts,
    Codes<Vec<u8>>,
    Contracts,
) {
    // TODO: create a `testdata` directory for the wasm files
    let codes = read_wasm_files(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts"))
        .unwrap();

    let db = DiskDb::open(dir).unwrap();
    let vm = WasmVm::new(wasm_cache_size);

    setup_suite_with_db_and_vm(db, vm, codes, NaiveProposalPreparer, NullIndexer)
}

/// Set up a test with the given DB, VM, and codes.
fn setup_suite_with_db_and_vm<DB, VM, T, PP, ID>(
    db: DB,
    vm: VM,
    codes: Codes<T>,
    pp: PP,
    indexer: ID,
) -> (TestSuite<PP, DB, VM, ID>, Accounts, Codes<T>, Contracts)
where
    T: Clone + Into<Binary>,
    DB: Db,
    VM: Vm + Clone,
    ID: Indexer,
    PP: grug_app::ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
    ID::Error: Display,
    DB::Error: Display,
{
    let owner = TestAccount::new_random("owner");
    let relayer = TestAccount::new_random("relayer");

    let (genesis_state, contracts, addresses) = build_genesis(
        codes.clone(),
        btree_map! {
            owner.username.clone() => GenesisUser {
                key: owner.key,
                key_hash: owner.key_hash,
                // Some of the tests depend on the number of tokens, so careful
                // when changing these. They may break tests...
                balances: btree_map! {
                    "udng"  => 100_000_000_000_000,
                    "uusdc" => 100_000_000_000,
                }
                .try_into()
                .unwrap(),
            },
            relayer.username.clone() => GenesisUser {
                key: relayer.key,
                key_hash: relayer.key_hash,
                balances: btree_map! {
                    "udng"  => 100_000_000_000_000,
                    "uusdc" => 100_000_000_000_000,
                    "uatom" => 100_000_000_000_000,
                    "uosmo" => 100_000_000_000_000,
                }
                .try_into()
                .unwrap(),
            },
        },
        &owner.username,
        TOKEN_FACTORY_CREATION_FEE.denom.to_string(),
        Udec128::ZERO,
        Some(TOKEN_FACTORY_CREATION_FEE.amount),
        Duration::from_seconds(7 * 24 * 60 * 60),
    )
    .unwrap();

    let suite = grug::TestSuite::new_with_db_vm_indexer_and_pp(
        db,
        vm,
        pp,
        indexer,
        CHAIN_ID.to_string(),
        Duration::from_millis(250),
        1_000_000,
        BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: GENESIS_TIMESTAMP,
        },
        genesis_state,
    );

    let accounts = Accounts {
        owner: owner.set_address(&addresses),
        relayer: relayer.set_address(&addresses),
    };

    (suite, accounts, codes, contracts)
}

fn build_codes() -> Codes<ContractWrapper> {
    let account_factory = ContractBuilder::new(Box::new(dango_account_factory::instantiate))
        .with_execute(Box::new(dango_account_factory::execute))
        .with_query(Box::new(dango_account_factory::query))
        .with_authenticate(Box::new(dango_account_factory::authenticate))
        .build();

    let account_margin = ContractBuilder::new(Box::new(dango_account_margin::instantiate))
        .with_authenticate(Box::new(dango_account_margin::authenticate))
        .with_backrun(Box::new(dango_account_margin::backrun))
        .with_receive(Box::new(dango_account_margin::receive))
        .with_query(Box::new(dango_account_margin::query))
        .build();

    let account_safe = ContractBuilder::new(Box::new(dango_account_safe::instantiate))
        .with_authenticate(Box::new(dango_account_safe::authenticate))
        .with_receive(Box::new(dango_account_safe::receive))
        .with_execute(Box::new(dango_account_safe::execute))
        .with_query(Box::new(dango_account_safe::query))
        .build();

    let account_spot = ContractBuilder::new(Box::new(dango_account_spot::instantiate))
        .with_authenticate(Box::new(dango_account_spot::authenticate))
        .with_receive(Box::new(dango_account_spot::receive))
        .with_query(Box::new(dango_account_spot::query))
        .build();

    let amm = ContractBuilder::new(Box::new(dango_amm::instantiate))
        .with_execute(Box::new(dango_amm::execute))
        .with_query(Box::new(dango_amm::query))
        .build();

    let bank = ContractBuilder::new(Box::new(dango_bank::instantiate))
        .with_execute(Box::new(dango_bank::execute))
        .with_query(Box::new(dango_bank::query))
        .with_bank_execute(Box::new(dango_bank::bank_execute))
        .with_bank_query(Box::new(dango_bank::bank_query))
        .build();

    let ibc_transfer = ContractBuilder::new(Box::new(dango_ibc_transfer::instantiate))
        .with_execute(Box::new(dango_ibc_transfer::execute))
        .build();

    let oracle = ContractBuilder::new(Box::new(dango_oracle::instantiate))
        .with_execute(Box::new(dango_oracle::execute))
        .with_authenticate(Box::new(dango_oracle::authenticate))
        .with_query(Box::new(dango_oracle::query))
        .build();

    let lending = ContractBuilder::new(Box::new(dango_lending::instantiate))
        .with_execute(Box::new(dango_lending::execute))
        .with_query(Box::new(dango_lending::query))
        .build();

    let taxman = ContractBuilder::new(Box::new(dango_taxman::instantiate))
        .with_execute(Box::new(dango_taxman::execute))
        .with_query(Box::new(dango_taxman::query))
        .with_withhold_fee(Box::new(dango_taxman::withhold_fee))
        .with_finalize_fee(Box::new(dango_taxman::finalize_fee))
        .build();

    let token_factory = ContractBuilder::new(Box::new(dango_token_factory::instantiate))
        .with_execute(Box::new(dango_token_factory::execute))
        .with_query(Box::new(dango_token_factory::query))
        .build();

    let vesting = ContractBuilder::new(Box::new(dango_vesting::instantiate))
        .with_execute(Box::new(dango_vesting::execute))
        .with_query(Box::new(dango_vesting::query))
        .build();

    Codes {
        account_factory,
        account_margin,
        account_safe,
        account_spot,
        amm,
        bank,
        ibc_transfer,
        lending,
        oracle,
        taxman,
        token_factory,
        vesting,
    }
}
