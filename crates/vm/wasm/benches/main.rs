use {
    criterion::{criterion_group, criterion_main, BenchmarkId, Criterion},
    grug_app::{GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_crypto::{sha2_256, Identity256},
    grug_tester_benchmarker::{CryptoApi, ExecuteTest},
    grug_types::{
        from_json_slice, to_borsh_vec, to_json_vec, Addr, BlockInfo, Coins, Context, GenericResult,
        Hash, Json, MockStorage, Timestamp,
    },
    grug_vm_wasm::{WasmInstance, WasmVm},
    k256::ecdsa::signature::DigestSigner,
    rand::{rngs::OsRng, RngCore},
    std::time::{Duration, Instant},
};

static BENCHMARKER: &[u8] = include_bytes!("../testdata/grug_tester_benchmarker.wasm");

// ------------------------------------ HELPERS ------------------------------------

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

fn execute_api(crypto_api: CryptoApi, on_host: bool, iters: u64) -> Duration {
    let mut total = Duration::ZERO;
    for _ in 0..iters {
        let (instance, context, _) = build_contract(BENCHMARKER);
        let start = Instant::now();
        let res = instance
            .call_in_1_out_1(
                "execute",
                &context,
                &to_json_vec(&ExecuteTest::Crypto {
                    on_host,
                    crypto_api: crypto_api.clone(),
                })
                .unwrap(),
            )
            .unwrap();

        total += start.elapsed();
        validate_ok_vm_result(res);
    }
    total
}

fn gen_random_msg(i: usize) -> Vec<u8> {
    let mut vec = vec![0; i];
    OsRng.fill_bytes(&mut vec);
    vec
}

fn validate_ok_vm_result(res: Vec<u8>) {
    from_json_slice::<GenericResult<Json>>(&res)
        .unwrap()
        .into_std_result()
        .unwrap();
}

// TODO: is worth to insert similar helpers inside grug-crypto crates for testing purpose?

struct CryptoGenerateDataResponse {
    pub hased_msg: Vec<u8>,
    pub pk: Vec<u8>,
    pub sig: Vec<u8>,
}

fn generate_secp256k1_data(prehash_msg: &[u8]) -> CryptoGenerateDataResponse {
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(prehash_msg));
    let sig: k256::ecdsa::Signature = sk.sign_digest(msg.clone());

    CryptoGenerateDataResponse {
        hased_msg: msg.as_bytes().to_vec(),
        pk: vk.to_sec1_bytes().to_vec(),
        sig: sig.to_vec(),
    }
}

// ------------------------------------ BENCHES ------------------------------------

fn wasmer_metering(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasmer_metering");

    for i in 1..20 {
        let mut gas = None;

        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, i| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let (instance, context, gas_tracker) = build_contract(BENCHMARKER);
                    let start = Instant::now();
                    let res = instance
                        .call_in_1_out_1(
                            "execute",
                            &context,
                            &to_json_vec(&ExecuteTest::Math {
                                iterations: *i as u64 * 1000_u64,
                            })
                            .unwrap(),
                        )
                        .unwrap();
                    total += start.elapsed();
                    validate_ok_vm_result(res);
                    if gas.is_none() {
                        gas = Some(gas_tracker.used());
                    }
                }
                total
            })
        });
    }

    group.finish();
}

fn serde_vs_borsh(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_vs_borsh");

    for i in [200_000, 700_000, 1_200_000, 1_700_000, 2_200_000] {
        let msg = gen_random_msg(i);

        group.bench_with_input(BenchmarkId::new("Serde", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let (instance, context, _) = build_contract(BENCHMARKER);
                    let start = Instant::now();
                    let res = instance
                        .call_in_1_out_1(
                            "execute",
                            &context,
                            &to_json_vec(&ExecuteTest::DoNothingBinary {
                                msg: msg.clone().into(),
                            })
                            .unwrap(),
                        )
                        .unwrap();

                    total += start.elapsed();
                    validate_ok_vm_result(res);
                }
                total
            })
        });

        group.bench_with_input(BenchmarkId::new("Borsh", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let (instance, context, _) = build_contract(BENCHMARKER);
                    let start = Instant::now();
                    let res = instance
                        .call_in_1_out_1(
                            "execute_borsh",
                            &context,
                            &to_borsh_vec(&ExecuteTest::DoNothingBinary {
                                msg: msg.clone().into(),
                            })
                            .unwrap(),
                        )
                        .unwrap();

                    total += start.elapsed();
                    validate_ok_vm_result(res);
                }
                total
            })
        });
    }
}

fn secp256k1_verify_host_vs_contract(c: &mut Criterion) {
    let mut group = c.benchmark_group("secp256k1_verify_host_vs_contract");

    for i in [10, 100, 1000, 10000] {
        let msg = gen_random_msg(i);
        let crypto_data = generate_secp256k1_data(&msg);

        group.bench_with_input(BenchmarkId::new("Host", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                execute_api(
                    CryptoApi::Sepc256k1verify {
                        msg_hash: crypto_data.hased_msg.clone(),
                        sig: crypto_data.sig.clone(),
                        pk: crypto_data.pk.clone(),
                    },
                    true,
                    iters,
                )
            })
        });

        group.bench_with_input(BenchmarkId::new("Contract", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                execute_api(
                    CryptoApi::Sepc256k1verify {
                        msg_hash: crypto_data.hased_msg.clone(),
                        sig: crypto_data.sig.clone(),
                        pk: crypto_data.pk.clone(),
                    },
                    false,
                    iters,
                )
            })
        });
    }
}

fn hashers_host_vs_contract(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashers_host_vs_contract");

    for i in [100, 1000, 5000, 10000] {
        let msg = gen_random_msg(i);

        group.bench_with_input(BenchmarkId::new("Sha256-Host", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                execute_api(CryptoApi::Sha2_256 { msg: msg.clone() }, true, iters)
            })
        });

        group.bench_with_input(BenchmarkId::new("Sha256-Contract", i), &i, |b, _i| {
            b.iter_custom(|iters| {
                execute_api(CryptoApi::Sha2_256 { msg: msg.clone() }, false, iters)
            })
        });

        group.bench_with_input(BenchmarkId::new("Blake3-Host", i), &i, |b, _i| {
            b.iter_custom(|iters| execute_api(CryptoApi::Blake3 { msg: msg.clone() }, true, iters))
        });

        group.bench_with_input(BenchmarkId::new("Blake3-Contract", i), &i, |b, _i| {
            b.iter_custom(|iters| execute_api(CryptoApi::Blake3 { msg: msg.clone() }, false, iters))
        });
    }
}

criterion_group!(
    name = bench_wasmer_metering;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(200);
    targets = wasmer_metering
);

criterion_group!(
    name = bench_serde_vs_borsh;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(200);
    targets = serde_vs_borsh
);

criterion_group!(
    name = bench_host_vs_contract;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = secp256k1_verify_host_vs_contract, hashers_host_vs_contract
);

criterion_main!(
    bench_wasmer_metering,
    bench_serde_vs_borsh,
    bench_host_vs_contract
);
