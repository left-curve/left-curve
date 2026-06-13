use {
    criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main},
    grug_app::{GasTracker, Instance, QuerierProviderImpl, StorageProvider, Vm},
    grug_crypto::sha2_256,
    grug_tester::QueryMsg,
    grug_types::{
        Addr, BlockInfo, BorshSerExt, Context, GenericResult, Hash, JsonSerExt, MockStorage,
        Timestamp,
    },
    grug_vm_wasm::WasmVm,
    std::time::Duration,
};

const MOCK_CHAIN_ID: &str = "dev-1";

const MOCK_CONTRACT: Addr = Addr::mock(1);

const MOCK_BLOCK: BlockInfo = BlockInfo {
    height: 1,
    timestamp: Timestamp::from_seconds(100),
    hash: Hash::ZERO,
};

static BENCHMARKER_CODE: &[u8] = include_bytes!("../testdata/grug_tester.wasm");

fn looping(c: &mut Criterion) {
    // Share one `WasmVm` across all benches, which caches the module, so we
    // don't need to rebuild it every time.
    let mut vm = WasmVm::new(100);

    for iterations in [200_000, 400_000, 600_000, 800_000, 1_000_000] {
        // The `criterion` library only benchmarks the time consumption, however
        // we additinally want to know the gas used, so that we can compute the
        // gas per second. So we record it separately here.
        let mut sum = 0;
        let mut repeats = 0;

        c.bench_with_input(
            BenchmarkId::new("looping", iterations),
            &iterations,
            |b, iterations| {
                // `Bencher::iter_with_setup` has been deprecated, in favor of
                // `Bencher::iter_batched` with `PerIteration`. See:
                // https://bheisler.github.io/criterion.rs/book/user_guide/timing_loops.html#deprecated-timing-loops
                b.iter_batched(
                    || {
                        let storage = Box::new(MockStorage::new());
                        let gas_tracker = GasTracker::new_limitless();

                        let querier = QuerierProviderImpl::new_boxed(
                            vm.clone(),
                            storage.clone(),
                            gas_tracker.clone(),
                            MOCK_BLOCK,
                        );
                        let storage = StorageProvider::new(storage, &[&MOCK_CONTRACT]);

                        let instance = vm
                            .build_instance(
                                BENCHMARKER_CODE,
                                Hash::from_inner(sha2_256(BENCHMARKER_CODE)),
                                storage,
                                true,
                                querier,
                                0,
                                gas_tracker.clone(),
                            )
                            .unwrap();

                        let ctx = Context {
                            chain_id: MOCK_CHAIN_ID.to_string(),
                            block: MOCK_BLOCK,
                            contract: MOCK_CONTRACT,
                            sender: None,
                            funds: None,
                            mode: None,
                        };

                        let msg = QueryMsg::Loop {
                            iterations: *iterations,
                        }
                        .to_json_value()
                        .unwrap()
                        .to_borsh_vec()
                        .unwrap();

                        let ok = GenericResult::Ok(().to_json_value().unwrap())
                            .to_borsh_vec()
                            .unwrap();

                        (instance, ctx, msg, ok, gas_tracker)
                    },
                    |suite| {
                        let (instance, ctx, msg, ok, gas_tracker) = suite;

                        // Call the `loop` query method
                        let output = instance.call_in_1_out_1("query", &ctx, &msg).unwrap();

                        // Make sure the contract didn't error
                        assert_eq!(output, ok);

                        // Record the gas consumed
                        sum += gas_tracker.used();
                        repeats += 1;
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        println!(
            "Iterations per run = {}; points per run = {}\n",
            iterations,
            sum / repeats
        );
    }
}

criterion_group! {
    name = wasmer_metering;
    config = Criterion::default().measurement_time(Duration::from_secs(40)).sample_size(200);
    targets = looping
}

criterion_main!(wasmer_metering);
