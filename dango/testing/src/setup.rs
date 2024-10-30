use {
    crate::{Accounts, TestAccount},
    dango_genesis::{build_genesis, read_wasm_files, Codes, Contracts, GenesisUser},
    grug::{
        btree_map, Binary, BlockInfo, Coins, ContractBuilder, ContractWrapper, Duration,
        NumberConst, TestSuite, Timestamp, Udec128, Uint128, GENESIS_BLOCK_HASH,
        GENESIS_BLOCK_HEIGHT,
    },
    grug_app::{AppError, Db, NaiveProposalPreparer, Vm},
    grug_db_disk::{DiskDb, TempDataDir},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    std::{env, path::PathBuf},
};

/// Set up a test with the given DB, VM, and codes.
fn setup_suite_with_db_and_vm<DB, VM, T>(
    db: DB,
    vm: VM,
    codes: Codes<T>,
) -> (TestSuite<DB, VM>, Accounts, Codes<T>, Contracts)
where
    T: Clone + Into<Binary>,
    DB: Db,
    VM: Vm + Clone,
    AppError: From<DB::Error> + From<VM::Error>,
{
    let owner = TestAccount::new_random("owner");
    let relayer = TestAccount::new_random("relayer");

    let (genesis_state, contracts, addresses) = build_genesis(
        codes.clone(),
        btree_map! {
            owner.username.clone() => GenesisUser {
                key: owner.key,
                key_hash: owner.key_hash,
                balances: Coins::one("uusdc", 100_000_000_000).unwrap(),
            },
            relayer.username.clone() => GenesisUser {
                key: relayer.key,
                key_hash: relayer.key_hash,
                balances: btree_map! {
                    "uusdc" => 100_000_000_000_000,
                    "uatom" => 100_000_000_000_000,
                    "uosmo" => 100_000_000_000_000,
                }
                .try_into()
                .unwrap(),
            },
        },
        &owner.username,
        "uusdc",
        Udec128::ZERO,
        Some(Uint128::new(10_000_000)),
        Duration::from_seconds(7 * 24 * 60 * 60),
    )
    .unwrap();

    let suite = TestSuite::new_with_db_vm_and_pp(
        db,
        vm,
        NaiveProposalPreparer,
        "dev-1".to_string(),
        Duration::from_millis(250),
        1_000_000,
        BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: Timestamp::from_seconds(0),
        },
        genesis_state,
    );

    let accounts = Accounts {
        owner: owner.set_address(&addresses),
        relayer: relayer.set_address(&addresses),
    };

    (suite, accounts, codes, contracts)
}

/// Set up a `TestSuite` with `MemDb`, `RustVm`, and `ContractWrapper` codes.
pub fn setup_test() -> (TestSuite, Accounts, Codes<ContractWrapper>, Contracts) {
    let account_factory = ContractBuilder::new(Box::new(dango_account_factory::instantiate))
        .with_execute(Box::new(dango_account_factory::execute))
        .with_query(Box::new(dango_account_factory::query))
        .with_authenticate(Box::new(dango_account_factory::authenticate))
        .build();

    let account_margin = ContractBuilder::new(Box::new(dango_account_margin::instantiate))
        .with_authenticate(Box::new(dango_account_margin::authenticate))
        .with_receive(Box::new(dango_account_margin::receive))
        .with_query(Box::new(dango_account_margin::query))
        .build();

    let account_spot = ContractBuilder::new(Box::new(dango_account_spot::instantiate))
        .with_authenticate(Box::new(dango_account_spot::authenticate))
        .with_receive(Box::new(dango_account_spot::receive))
        .with_query(Box::new(dango_account_spot::query))
        .build();

    let account_safe = ContractBuilder::new(Box::new(dango_account_safe::instantiate))
        .with_authenticate(Box::new(dango_account_safe::authenticate))
        .with_receive(Box::new(dango_account_safe::receive))
        .with_execute(Box::new(dango_account_safe::execute))
        .with_query(Box::new(dango_account_safe::query))
        .build();

    let amm = ContractBuilder::new(Box::new(dango_amm::instantiate))
        .with_execute(Box::new(dango_amm::execute))
        .with_query(Box::new(dango_amm::query))
        .build();

    let bank = ContractBuilder::new(Box::new(dango_bank::instantiate))
        .with_execute(Box::new(dango_bank::execute))
        .with_bank_execute(Box::new(dango_bank::bank_execute))
        .with_bank_query(Box::new(dango_bank::bank_query))
        .build();

    let ibc_transfer = ContractBuilder::new(Box::new(dango_ibc_transfer::instantiate))
        .with_execute(Box::new(dango_ibc_transfer::execute))
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

    let codes = Codes {
        account_factory,
        account_margin,
        account_spot,
        account_safe,
        amm,
        bank,
        ibc_transfer,
        lending,
        taxman,
        token_factory,
    };

    setup_suite_with_db_and_vm(MemDb::new(), RustVm::new(), codes)
}

/// Set up a `TestSuite` with `DiskDb`, `WasmVm`, and `Vec<u8>` codes.
/// Used for benchmarks.
pub fn setup_benchmark(
    dir: &TempDataDir,
    wasm_cache_size: usize,
) -> (
    TestSuite<DiskDb, WasmVm>,
    Accounts,
    Codes<Vec<u8>>,
    Contracts,
) {
    // TODO: create a `testdata` directory for the wasm files
    let codes = read_wasm_files(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts"))
        .unwrap();

    let db = DiskDb::open(dir).unwrap();
    let vm = WasmVm::new(wasm_cache_size);

    setup_suite_with_db_and_vm(db, vm, codes)
}
