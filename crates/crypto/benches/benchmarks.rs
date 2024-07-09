use {
    criterion::{
        black_box, criterion_group, criterion_main, AxisScale, BatchSize, BenchmarkId, Criterion,
        PlotConfiguration,
    },
    ed25519_dalek::Signer,
    grug_crypto::{
        blake2b_512, blake2s_256, blake3, ed25519_batch_verify, ed25519_verify, keccak256,
        secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256, sha2_512, sha3_256,
        sha3_512, Identity256, Identity512,
    },
    p256::ecdsa::signature::DigestSigner,
    rand::{rngs::OsRng, RngCore},
    std::time::Duration,
};

struct Settings {
    iter: usize,
    mul_iter: usize,
    warmup_time: Duration,
    measurement_time: Duration,
}

const HASH_SETTINGS: Settings = Settings {
    iter: 30,
    mul_iter: 10_000,
    warmup_time: Duration::from_millis(100),
    measurement_time: Duration::from_millis(100),
};

const CRYPTO_SETTINGS: Settings = Settings {
    iter: 10,
    mul_iter: 20,
    warmup_time: Duration::from_millis(2_000),
    measurement_time: Duration::from_millis(2_000),
};

fn generate_random_msg(i: usize) -> Vec<u8> {
    let mut vec = vec![0; i];
    OsRng.fill_bytes(&mut vec);
    vec
}

fn bench_hashers(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashers");

    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
    group.warm_up_time(HASH_SETTINGS.warmup_time);
    group.measurement_time(HASH_SETTINGS.measurement_time);

    for size in (1..=HASH_SETTINGS.iter).map(|size| size * HASH_SETTINGS.mul_iter) {
        group.bench_with_input(BenchmarkId::new("sha2_256", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| sha2_256(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("sha2_512", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| sha2_512(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("sha3_256", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| sha3_256(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("sha3_512", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| sha3_512(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("keccak256", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| keccak256(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("blake2s_256", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| blake2s_256(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("blake2b_512", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| blake2b_512(black_box(&data)),
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("blake3", size), &size, |b, size| {
            b.iter_batched(
                || generate_random_msg(*size),
                |data| blake3(black_box(&data)),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_verifiers(c: &mut Criterion) {
    const MSG_LEN: usize = 100;

    let mut group = c.benchmark_group("verifiers");

    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
    group.warm_up_time(CRYPTO_SETTINGS.warmup_time);
    group.measurement_time(CRYPTO_SETTINGS.measurement_time);

    group.bench_function("secp256r1_verify", |b| {
        b.iter_batched(
            || {
                let msg = generate_random_msg(MSG_LEN);
                let msg_hash = Identity256::from(sha2_256(&msg));
                let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
                let vk = p256::ecdsa::VerifyingKey::from(&sk);
                let sig: p256::ecdsa::Signature = sk.sign_digest(msg_hash.clone()).unwrap();

                (
                    msg_hash.as_bytes().to_vec(),
                    sig.to_bytes().to_vec(),
                    vk.to_sec1_bytes().to_vec(),
                )
            },
            |(msg_hash, sig, vk)| {
                assert!(secp256r1_verify(&msg_hash, &sig, &vk).is_ok());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("secp256k1_verify", |b| {
        b.iter_batched(
            || {
                let msg = generate_random_msg(MSG_LEN);
                let msg_hash = Identity256::from(sha2_256(&msg));
                let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
                let vk = k256::ecdsa::VerifyingKey::from(&sk);
                let sig: k256::ecdsa::Signature = sk.sign_digest(msg_hash.clone()).unwrap();

                (
                    msg_hash.as_bytes().to_vec(),
                    sig.to_bytes().to_vec(),
                    vk.to_sec1_bytes().to_vec(),
                )
            },
            |(msg_hash, sig, vk)| {
                assert!(secp256k1_verify(&msg_hash, &sig, &vk).is_ok());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("secp256k1_pubkey_recover", |b| {
        b.iter_batched(
            || {
                let msg = generate_random_msg(MSG_LEN);
                let msg_hash = Identity256::from(sha2_256(&msg));
                let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
                let vk = k256::ecdsa::VerifyingKey::from(&sk);
                let (sig, recovery_id) = sk.sign_digest_recoverable(msg_hash.clone()).unwrap();

                (
                    msg_hash.as_bytes().to_vec(),
                    sig.to_bytes().to_vec(),
                    vk.to_sec1_bytes().to_vec(),
                    recovery_id.to_byte(),
                    true,
                )
            },
            |(msg_hash, sig, vk, recovery_id, compressed)| {
                assert_eq!(
                    secp256k1_pubkey_recover(&msg_hash, &sig, recovery_id, compressed).unwrap(),
                    vk
                );
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("ed25519_verify", |b| {
        b.iter_batched(
            || {
                let msg = generate_random_msg(MSG_LEN);
                let msg_hash = Identity512::from(sha2_512(&msg));
                let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
                let vk = ed25519_dalek::VerifyingKey::from(&sk);
                let sig = sk.sign_digest(msg_hash.clone());

                (
                    msg_hash.to_vec(),
                    sig.to_bytes().to_vec(),
                    vk.as_bytes().to_vec(),
                )
            },
            |(msg_hash, sig, vk)| {
                assert!(ed25519_verify(&msg_hash, &sig, &vk).is_ok());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("ed25519_batch_verify", |b| {
        b.iter_batched(
            || {
                let mut prehash_msgs = vec![];
                let mut sigs = vec![];
                let mut vks = vec![];

                // We use a batch size of 100, because the typical use case of
                // `ed25519_batch_verify` is in Tendermint light clients, while
                // the average size of validator sets of Cosmos chains is ~100.
                for _ in 0..100 {
                    let prehash_msg = generate_random_msg(MSG_LEN);
                    let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
                    let vk = ed25519_dalek::VerifyingKey::from(&sk);
                    let sig = sk.sign(&prehash_msg);

                    prehash_msgs.push(prehash_msg);
                    sigs.push(sig.to_bytes().to_vec());
                    vks.push(vk.to_bytes().to_vec());
                }

                (prehash_msgs, sigs, vks)
            },
            |(prehash_msgs, sigs, vks)| {
                let prehash_msgs: Vec<_> = prehash_msgs.iter().map(|m| m.as_slice()).collect();
                let sigs: Vec<_> = sigs.iter().map(|s| s.as_slice()).collect();
                let vks: Vec<_> = vks.iter().map(|k| k.as_slice()).collect();
                assert!(ed25519_batch_verify(&prehash_msgs, &sigs, &vks).is_ok());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_hashers, bench_verifiers);

criterion_main!(benches);
