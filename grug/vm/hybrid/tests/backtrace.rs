use {
    grug_db_memory::MemDb,
    grug_math::Udec128,
    grug_tester::{
        BacktraceQueryResponse, QueryBacktraceRequest, QueryFailingQueryRequest, QueryMsg,
    },
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{
        Addr, Backtraceable, Binary, Coins, Denom, HashExt, QuerierExt, Query, ResultExt,
    },
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::ContractBuilder,
    std::{fs, path::PathBuf, str::FromStr, sync::LazyLock},
};

const WASM_CACHE_CAPACITY: usize = 10;

const FEE_RATE: Udec128 = Udec128::new_percent(10);

fn read_wasm_file(filename: &str) -> Binary {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("wasm");
    path.push("testdata");
    path.push(filename);

    fs::read(path).unwrap().into()
}

static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("ugrug").unwrap());

pub fn setup_test() -> (TestSuite<MemDb, HybridVm>, TestAccounts, Addr, Addr) {
    let rust_tester: Binary = ContractBuilder::new(Box::new(grug_tester::instantiate))
        .with_query(Box::new(grug_tester::query))
        .build()
        .to_bytes()
        .into();

    let vm = HybridVm::new_testing(WASM_CACHE_CAPACITY, [rust_tester.hash256()]);

    let (mut suite, mut accounts) = TestBuilder::new_with_vm(vm)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(DENOM.clone(), 300_000_000).unwrap())
        .set_owner("owner")
        .set_fee_rate(FEE_RATE)
        .build();

    let wasm_tester = suite
        .upload_and_instantiate_with_gas(
            &mut accounts["sender"],
            320_000_000,
            read_wasm_file("grug_tester.wasm"),
            &grug_tester::InstantiateMsg {},
            "tester",
            Some("tester"),
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    let rust_tester = suite
        .upload_and_instantiate_with_gas(
            &mut accounts["sender"],
            100_000,
            rust_tester,
            &grug_tester::InstantiateMsg {},
            "tester",
            Some("tester"),
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    (suite, accounts, wasm_tester, rust_tester)
}

#[test]
fn backtrace() {
    let (suite, _, wasm_tester, rust_tester) = setup_test();

    let res = suite
        .query_wasm_smart(wasm_tester, QueryBacktraceRequest {
            query: Query::wasm_smart(rust_tester, &QueryMsg::FailingQuery {
                msg: "boom".to_string(),
            })
            .unwrap(),
        })
        .should_succeed();

    if let BacktraceQueryResponse::Err(err) = res {
        assert_eq!(
            err.error,
            "host returned error: contract returned error! address: 0xb304e60745d4cda6b1bf3248b979545522e9ccc1, method: query, msg: host returned error: boom"
        );
        assert_eq!(err.backtrace.to_string(), "")
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

    // on wasm, backtrace is empty but the chain capture the backtrace later.
    // there are not any grug_tester word in the backtrace.
    assert!(!backtrace.is_empty());
    assert!(!backtrace.contains("grug_tester"));

    let backtrace = suite
        .query_wasm_smart(rust_tester, QueryFailingQueryRequest {
            msg: "boom rust".to_string(),
        })
        .unwrap_err()
        .into_generic_backtraced_error()
        .backtrace
        .to_string();

    assert!(!backtrace.is_empty());
    assert!(backtrace.contains("grug_tester"));
}
