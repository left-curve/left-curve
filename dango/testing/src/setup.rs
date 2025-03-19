use {
    crate::{TestAccount, TestAccounts, MOCK_LOCAL_DOMAIN},
    dango_app::ProposalPreparer,
    dango_genesis::{
        build_genesis, build_rust_codes, read_wasm_files, Codes, Contracts, GenesisConfig,
        GenesisUser,
    },
    dango_types::{
        constants::{
            BTC_DENOM, DANGO_DENOM, ETH_DENOM, PYTH_PRICE_SOURCES, SOL_DENOM, USDC_DENOM,
            WBTC_DENOM,
        },
        dex::{CurveInvariant, PairParams, PairUpdate},
        lending::InterestRateModel,
        taxman,
    },
    grug::{
        btree_map, coins, Binary, BlockInfo, Bounded, Coin, ContractWrapper, Denom, Duration,
        HashExt, NumberConst, Timestamp, Udec128, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT,
    },
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, Vm},
    grug_db_disk::{DiskDb, TempDataDir},
    grug_db_memory::MemDb,
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    hex_literal::hex,
    indexer_httpd::context::Context,
    indexer_sql::non_blocking_indexer::NonBlockingIndexer,
    pyth_client::PythClientCache,
    pyth_types::GUARDIAN_SETS,
    std::{path::PathBuf, str::FromStr, sync::Arc},
};

pub const MOCK_CHAIN_ID: &str = "mock-1";
pub const MOCK_GENESIS_TIMESTAMP: Timestamp = Timestamp::from_days(365);

pub const OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");
pub const USER1_PRIVATE_KEY: [u8; 32] =
    hex!("a5122c0729c1fae8587e3cc07ae952cb77dfccc049efd5be1d2168cbe946ca18");
pub const USER2_PRIVATE_KEY: [u8; 32] =
    hex!("cac7b4ced59cf0bfb14c373272dfb3d4447c7cd5aea732ea6ff69e19f85d34c4");
pub const USER3_PRIVATE_KEY: [u8; 32] =
    hex!("cf6bb15648a3a24976e2eeffaae6201bc3e945335286d273bb491873ac7c3141");
pub const USER4_PRIVATE_KEY: [u8; 32] =
    hex!("126b714bfe7ace5aac396aa63ff5c92c89a2d894debe699576006202c63a9cf6");
pub const USER5_PRIVATE_KEY: [u8; 32] =
    hex!("fe55076e4b2c9ffea813951406e8142fefc85183ebda6222500572b0a92032a7");
pub const USER6_PRIVATE_KEY: [u8; 32] =
    hex!("4d3658519dd8a8227764f64c6724b840ffe29f1ca456f5dfdd67f834e10aae34");
pub const USER7_PRIVATE_KEY: [u8; 32] =
    hex!("82de24ba8e1bc4511ae10ce3fbe84b4bb8d7d8abc9ba221d7d3cf7cd0a85131f");
pub const USER8_PRIVATE_KEY: [u8; 32] =
    hex!("ca956fcf6b0f32975f067e2deaf3bc1c8632be02ed628985105fd1afc94531b9");
pub const USER9_PRIVATE_KEY: [u8; 32] =
    hex!("c0d853951557d3bdec5add2ca8e03983fea2f50c6db0a45977990fb7b0c569b3");

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

/// Set up a `TestSuite` with `MemDb`, `RustVm`, `ProposalPreparer`, and
/// `ContractWrapper` codes.
///
/// Used for running regular tests.
pub fn setup_test() -> (TestSuite, TestAccounts, Codes<ContractWrapper>, Contracts) {
    let codes = build_rust_codes();

    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        codes,
        ProposalPreparer::new_with_cache(),
        NullIndexer,
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
    let codes = build_rust_codes();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_hooks(dango_indexer_sql::hooks::Hooks)
        .build()
        .unwrap();

    let indexer_context = indexer.context.clone();

    let db = MemDb::new();
    let vm = RustVm::new();

    let (suite, accounts, codes, contracts) = setup_suite_with_db_and_vm(
        db.clone(),
        vm.clone(),
        codes,
        ProposalPreparer::new_with_cache(),
        indexer,
    );

    let httpd_context = Context::new(
        indexer_context,
        Arc::new(suite.app.clone_without_indexer()),
        "http://localhost:26657",
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
    let codes = build_rust_codes();

    setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        codes,
        NaiveProposalPreparer,
        NullIndexer,
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
    let codes = build_rust_codes();
    let db = DiskDb::open(dir).unwrap();
    let vm = HybridVm::new(wasm_cache_size, [
        codes.account_factory.to_bytes().hash256(),
        codes.account_margin.to_bytes().hash256(),
        codes.account_multi.to_bytes().hash256(),
        codes.account_spot.to_bytes().hash256(),
        codes.bank.to_bytes().hash256(),
        codes.lending.to_bytes().hash256(),
        codes.oracle.to_bytes().hash256(),
        codes.taxman.to_bytes().hash256(),
        codes.vesting.to_bytes().hash256(),
    ]);

    setup_suite_with_db_and_vm(db, vm, codes, NaiveProposalPreparer, NullIndexer)
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
    let codes = read_wasm_files(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts"))
        .unwrap();
    let db = DiskDb::open(dir).unwrap();
    let vm = WasmVm::new(wasm_cache_size);

    setup_suite_with_db_and_vm(db, vm, codes, NaiveProposalPreparer, NullIndexer)
}

fn setup_suite_with_db_and_vm<DB, VM, T, PP, ID>(
    db: DB,
    vm: VM,
    codes: Codes<T>,
    pp: PP,
    indexer: ID,
) -> (TestSuite<PP, DB, VM, ID>, TestAccounts, Codes<T>, Contracts)
where
    T: Clone + Into<Binary>,
    DB: Db,
    VM: Vm + Clone + 'static,
    ID: Indexer,
    PP: grug_app::ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    let owner = TestAccount::new_from_private_key("owner", OWNER_PRIVATE_KEY);
    let user1 = TestAccount::new_from_private_key("user1", USER1_PRIVATE_KEY);
    let user2 = TestAccount::new_from_private_key("user2", USER2_PRIVATE_KEY);
    let user3 = TestAccount::new_from_private_key("user3", USER3_PRIVATE_KEY);
    let user4 = TestAccount::new_from_private_key("user4", USER4_PRIVATE_KEY);
    let user5 = TestAccount::new_from_private_key("user5", USER5_PRIVATE_KEY);
    let user6 = TestAccount::new_from_private_key("user6", USER6_PRIVATE_KEY);
    let user7 = TestAccount::new_from_private_key("user7", USER7_PRIVATE_KEY);
    let user8 = TestAccount::new_from_private_key("user8", USER8_PRIVATE_KEY);
    let user9 = TestAccount::new_from_private_key("user9", USER9_PRIVATE_KEY);

    let (genesis_state, contracts, addresses) = build_genesis(GenesisConfig {
        codes: codes.clone(),
        users: btree_map! {
            owner.username.clone() => GenesisUser {
                key: owner.key(),
                key_hash: owner.key_hash(),
                // Some of the tests depend on the number of tokens, so careful
                // when changing these. They may break tests...
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000,
                },
            },
            user1.username.clone() => GenesisUser {
                key: user1.key(),
                key_hash: user1.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                    // In reality, it's not possible that anyone has Hyperlane
                    // synth tokens in genesis. We add this just for testing purpose.
                    WBTC_DENOM.clone() => 100_000_000_000_000,
                    ETH_DENOM.clone()  => 100_000_000_000_000,
                }
            },
            user2.username.clone() => GenesisUser {
                key: user2.key(),
                key_hash: user2.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user3.username.clone() => GenesisUser {
                key: user3.key(),
                key_hash: user3.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user4.username.clone() => GenesisUser {
                key: user4.key(),
                key_hash: user4.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user5.username.clone() => GenesisUser {
                key: user5.key(),
                key_hash: user5.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user6.username.clone() => GenesisUser {
                key: user6.key(),
                key_hash: user6.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user7.username.clone() => GenesisUser {
                key: user7.key(),
                key_hash: user7.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user8.username.clone() => GenesisUser {
                key: user8.key(),
                key_hash: user8.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            user9.username.clone() => GenesisUser {
                key: user9.key(),
                key_hash: user9.key_hash(),
                balances: coins! {
                    DANGO_DENOM.clone() => 100_000_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
        },
        account_factory_minimum_deposit: coins! { USDC_DENOM.clone() => 10_000_000 },
        owner: owner.username.clone(),
        fee_cfg: taxman::Config {
            fee_denom: USDC_DENOM.clone(),
            fee_rate: Udec128::ZERO,
        },
        max_orphan_age: Duration::from_seconds(7 * 24 * 60 * 60),
        metadatas: btree_map! {},
        pairs: vec![
            PairUpdate {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO), // TODO: set to non-zero
                },
            },
            PairUpdate {
                base_denom: BTC_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO), // TODO: set to non-zero
                },
            },
            PairUpdate {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/eth/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO), // TODO: set to non-zero
                },
            },
            PairUpdate {
                base_denom: SOL_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/sol/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO), // TODO: set to non-zero
                },
            },
        ],
        markets: btree_map! {
            USDC_DENOM.clone() => InterestRateModel::default(),
            WBTC_DENOM.clone() => InterestRateModel::default(),
        },
        price_sources: PYTH_PRICE_SOURCES.clone(),
        unlocking_cliff: Duration::from_weeks(4 * 9),
        unlocking_period: Duration::from_weeks(4 * 27),
        wormhole_guardian_sets: GUARDIAN_SETS.clone(),
        hyperlane_local_domain: MOCK_LOCAL_DOMAIN,
        hyperlane_ism_validator_sets: btree_map! {},
        hyperlane_va_announce_fee_per_byte: Coin::new(USDC_DENOM.clone(), 100).unwrap(),
        warp_routes: btree_map! {},
    })
    .unwrap();

    let suite = grug::TestSuite::new_with_db_vm_indexer_and_pp(
        db,
        vm,
        pp,
        indexer,
        MOCK_CHAIN_ID.to_string(),
        Duration::from_millis(250),
        1_000_000,
        BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: MOCK_GENESIS_TIMESTAMP,
        },
        genesis_state,
    );

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
