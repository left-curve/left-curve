use {
    criterion::{criterion_group, criterion_main, BenchmarkId, Criterion},
    grug_app::{GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_crypto::sha2_256,
    grug_tester_benchmarker::TestConfig,
    grug_types::{to_json_vec, Addr, BlockInfo, Coins, Context, Hash, MockStorage, Timestamp},
    grug_vm_wasm::{WasmInstance, WasmVm},
    std::time::{Duration, Instant},
};

static BENCHMARKER: &[u8] = include_bytes!("../testdata/grug_tester_benchmarker.wasm");

fn build_contract(code: &[u8]) -> (WasmInstance, Context, GasTracker) {
    let block = BlockInfo {
        height: 1_u64.into(),
        timestamp: Timestamp::from_seconds(100),
        hash: Hash::ZERO,
    };

    let gas_tracker = GasTracker::new_limitless();
    let mut vm = WasmVm::new(100);
    let mock_storage = Box::new(MockStorage::new());
    let addr = Addr::mock(1);
    let storage_provider = StorageProvider::new(mock_storage.clone(), &[&addr]);
    let querier_provider = QuerierProvider::new(
        vm.clone(),
        mock_storage.clone(),
        gas_tracker.clone(),
        block.clone(),
    );
    let istance = vm
        .build_instance(
            code,
            &Hash::from_slice(sha2_256(code)),
            storage_provider,
            false,
            querier_provider,
            gas_tracker.clone(),
        )
        .unwrap();

    let context = Context {
        chain_id: "grug-1".to_string(),
        block,
        contract: addr,
        sender: Some(Addr::mock(2)),
        funds: Some(Coins::default()),
        simulate: None,
    };

    (istance, context, gas_tracker)
}

fn bench_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("instantiate");

    for i in 1..20 {
        let mut gas = None;

        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, i| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let (instance, context, gas_tracker) = build_contract(BENCHMARKER);
                    let start = Instant::now();
                    instance
                        .call_in_1_out_1(
                            "execute",
                            &context,
                            &to_json_vec(&TestConfig {
                                iterations: *i as u64 * 1000_u64,
                                debug: false,
                            })
                            .unwrap(),
                        )
                        .unwrap();
                    total += start.elapsed();
                    if gas.is_none() {
                        gas = Some(gas_tracker.used());
                    }
                }
                total
            })
        });

        println!("gas: {:?}", gas.unwrap());
    }

    group.finish();
}

criterion_group!(
    name = multi_threaded_instance;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = bench_instance
);
criterion_main!(multi_threaded_instance);
