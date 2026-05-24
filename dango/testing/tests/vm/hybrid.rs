use {
    dango_genesis::{GenesisCodes, GenesisOption},
    dango_testing::{Preset, TestAccounts, TestSuite, setup_suite_with_db_and_vm},
    error_backtrace::Backtraceable,
    grug_app::{NaiveProposalPreparer, NullIndexer},
    grug_db_memory::MemDb,
    grug_tester::{
        BacktraceQueryResponse, QueryBacktraceRequest, QueryFailingQueryRequest, QueryMsg,
    },
    grug_types::{Addr, Binary, Coins, HashExt, QuerierExt, Query, ResultExt},
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::{ContractBuilder, RustVm},
    std::fs,
};

const WASM_CACHE_CAPACITY: usize = 10;

fn read_wasm_file(filename: &str) -> Binary {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).unwrap().into()
}

pub async fn setup_test() -> (
    TestSuite<MemDb, HybridVm, NaiveProposalPreparer>,
    TestAccounts,
    Addr,
    Addr,
) {
    let rust_tester: Binary = ContractBuilder::new(Box::new(grug_tester::instantiate))
        .with_query(Box::new(grug_tester::query))
        .build()
        .into();

    let codes = RustVm::genesis_codes();

    let vm = HybridVm::new(WASM_CACHE_CAPACITY, {
        let mut rust_hashes = codes.all_code_hashes();
        rust_hashes.insert(rust_tester.hash256());
        rust_hashes
    });

    let (mut suite, mut accounts, ..) = setup_suite_with_db_and_vm(
        MemDb::new(),
        vm,
        NaiveProposalPreparer,
        NullIndexer,
        codes,
        Default::default(),
        GenesisOption::preset_test(),
    );

    let wasm_tester = suite
        .upload_and_instantiate_with_gas(
            &mut accounts.owner,
            320_000_000,
            read_wasm_file("grug_tester.wasm"),
            &grug_tester::InstantiateMsg {},
            "tester",
            Some("tester"),
            None,
            Coins::new(),
        )
        .await
        .should_succeed()
        .address;

    let rust_tester = suite
        .upload_and_instantiate_with_gas(
            &mut accounts.owner,
            100_000,
            rust_tester,
            &grug_tester::InstantiateMsg {},
            "tester",
            Some("tester"),
            None,
            Coins::new(),
        )
        .await
        .should_succeed()
        .address;

    (suite, accounts, wasm_tester, rust_tester)
}

async fn do_backtrace_test() {
    let (suite, _, wasm_tester, rust_tester) = setup_test().await;

    let res = suite
        .query_wasm_smart(wasm_tester, QueryBacktraceRequest {
            query: Query::wasm_smart(rust_tester, &QueryMsg::FailingQuery {
                msg: "boom".to_string(),
            })
            .unwrap(),
        })
        .should_succeed();

    if let BacktraceQueryResponse::Err(err) = res {
        let expected = format!(
            "host returned error: contract returned error! address: {rust_tester}, method: query, msg: host returned error: boom"
        );
        assert_eq!(err.error, expected);
        assert_eq!(err.backtrace.to_string(), "");
    } else {
        panic!("expected error");
    }

    let backtrace = suite
        .query_wasm_smart(wasm_tester, QueryFailingQueryRequest {
            msg: "boom wasm".to_string(),
        })
        .unwrap_err()
        .into_generic_backtraced_error()
        .backtrace
        .to_string();

    assert!(!backtrace.is_empty());
    assert!(!backtrace.contains("grug_tester::query::failing_query"));

    let backtrace = suite
        .query_wasm_smart(rust_tester, QueryFailingQueryRequest {
            msg: "boom rust".to_string(),
        })
        .unwrap_err()
        .into_generic_backtraced_error()
        .backtrace
        .to_string();

    assert!(!backtrace.is_empty());
    assert!(backtrace.contains("grug_tester::query::failing_query"));
}

#[test]
fn backtrace() {
    // Must set RUST_BACKTRACE before spawning the tokio runtime to avoid
    // unsound concurrent env mutation.
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(do_backtrace_test());
}
