use {
    crate::{Accounts, TestAccount},
    dango_app::ProposalPreparer,
    dango_genesis::{build_genesis, build_rust_codes, Codes, Contracts, GenesisUser},
    grug::{
        btree_map, Binary, BlockInfo, Coin, ContractWrapper, Duration, HashExt, NumberConst,
        Timestamp, Udec128, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, Vm},
    grug_db_disk::{DiskDb, TempDataDir},
    grug_db_memory::MemDb,
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::RustVm,
    std::sync::LazyLock,
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
    let codes = build_rust_codes();

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
    let codes = build_rust_codes();

    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        codes,
        NaiveProposalPreparer,
        NullIndexer,
    )
}

/// Set up a `TestSuite` with `DiskDb`, `HybridVm`, and `ContractWrapper` codes.
/// Used for benchmarks.
pub fn setup_benchmark(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<NaiveProposalPreparer, DiskDb, HybridVm, NullIndexer>,
    Accounts,
    Codes<ContractWrapper>,
    Contracts,
) {
    let codes = build_rust_codes();
    let db = DiskDb::open(dir).unwrap();
    let vm = HybridVm::new(wasm_cache_size, [
        codes.account_factory.to_bytes().hash256(),
        codes.account_margin.to_bytes().hash256(),
        codes.account_safe.to_bytes().hash256(),
        codes.account_spot.to_bytes().hash256(),
        codes.amm.to_bytes().hash256(),
        codes.bank.to_bytes().hash256(),
        codes.ibc_transfer.to_bytes().hash256(),
        codes.lending.to_bytes().hash256(),
        codes.oracle.to_bytes().hash256(),
        codes.taxman.to_bytes().hash256(),
        codes.token_factory.to_bytes().hash256(),
        codes.vesting.to_bytes().hash256(),
    ]);

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
    VM: Vm + Clone + 'static,
    ID: Indexer,
    PP: grug_app::ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    let owner = TestAccount::new_random("owner");
    let relayer = TestAccount::new_random("relayer");

    let (genesis_state, contracts, addresses) = build_genesis(
        codes.clone(),
        btree_map! {
            owner.username.clone() => GenesisUser {
                key: *owner.key(),
                key_hash: owner.key_hash(),
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
                key: *relayer.key(),
                key_hash: relayer.key_hash(),
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
