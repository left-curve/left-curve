use {
    criterion::{
        AxisScale, BatchSize, BenchmarkId, Criterion, PlotConfiguration, criterion_group,
        criterion_main,
    },
    ed25519_dalek::Signer,
    grug_crypto::{
        blake2b_512, blake2s_256, blake3, ed25519_batch_verify, ed25519_verify, keccak256,
        secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256, sha2_512, sha3_256,
        sha3_512,
    },
    identity::{Identity256, Identity512},
    p256::ecdsa::signature::DigestSigner,
    rand::{RngCore, rngs::OsRng},
    std::{hint::black_box, time::Duration},
};

struct Settings {
    warmup_time: Duration,
    measurement_time: Duration,
}

const HASH_SETTINGS: Settings = Settings {
    warmup_time: Duration::from_millis(500),
    measurement_time: Duration::from_millis(1_500),
};

const CRYPTO_SETTINGS: Settings = Settings {
    warmup_time: Duration::from_millis(2_000),
    measurement_time: Duration::from_millis(5_000),
};

/// Lengths of messages for benchmarking hashers.
///
/// In Grug contracts, the largest data that may need to hashed are transactions
/// that contain `Message::Upload` messages. Contract size are usually a few
/// hundred kilobytes, so here we choose messages of up to 1 MiB.
const HASH_MSG_LENS: [usize; 5] = [200_000, 400_000, 600_000, 800_000, 1_000_000];

/// Messages to be signed for benchmarking verifiers.
///
/// This length doesn't matter, because verifiers only concern hashes, so we
/// just pick a small number.
const SIGN_MSG_LEN: usize = 10;

/// Batch sizes for benchmarking `ed25519_batch_verify`.
///
/// The most common situation this function is called is in the ICS-07 Tendermint
/// light client, where it verifies block headers. Cosmos chains usually have up
/// to 150 validators, so we choose a number of batch sizes up to that.
const ED25519_BATCH_SIZES: [usize; 6] = [25, 50, 75, 100, 125, 150];

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

    for size in HASH_MSG_LENS {
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
    let mut group = c.benchmark_group("verifiers");

    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
    group.warm_up_time(CRYPTO_SETTINGS.warmup_time);
    group.measurement_time(CRYPTO_SETTINGS.measurement_time);

    group.bench_function("secp256r1_verify", |b| {
        b.iter_batched(
            || {
                let msg = generate_random_msg(SIGN_MSG_LEN);
                let msg_hash = Identity256::from(sha2_256(&msg));
                let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
                let vk = p256::ecdsa::VerifyingKey::from(&sk);
                // For some reason, we have to explicitly specify the trait here;
                // the compiler can't infer it.
                let sig = <p256::ecdsa::SigningKey as DigestSigner<_, p256::ecdsa::Signature>>::sign_digest(
                    &sk,
                    msg_hash.clone(),
                );

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
                let msg = generate_random_msg(SIGN_MSG_LEN);
                let msg_hash = Identity256::from(sha2_256(&msg));
                let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
                let vk = k256::ecdsa::VerifyingKey::from(&sk);
                let sig = <k256::ecdsa::SigningKey as DigestSigner<_, k256::ecdsa::Signature>>::sign_digest(
                    &sk,
                    msg_hash.clone(),
                );

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
                let msg = generate_random_msg(SIGN_MSG_LEN);
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
                let msg = generate_random_msg(SIGN_MSG_LEN);
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

    for size in ED25519_BATCH_SIZES {
        group.bench_with_input(
            BenchmarkId::new("ed25519_batch_verify", size),
            &size,
            |b, size| {
                b.iter_batched(
                    || {
                        let mut prehash_msgs = vec![];
                        let mut sigs = vec![];
                        let mut vks = vec![];

                        // We use a batch size of 100, because the typical use case of
                        // `ed25519_batch_verify` is in Tendermint light clients, while
                        // the average size of validator sets of Cosmos chains is ~100.
                        for _ in 0..*size {
                            let prehash_msg = generate_random_msg(SIGN_MSG_LEN);
                            let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
                            let vk = ed25519_dalek::VerifyingKey::from(&sk);
                            let sig = sk.sign(&prehash_msg);

                            prehash_msgs.push(prehash_msg);
                            sigs.push(sig.to_bytes().to_vec());
                            vks.push(vk.to_bytes().to_vec());
                        }

                        (prehash_msgs, sigs, vks)
                    },
                    |(msgs, sigs, vks)| {
                        let msgs: Vec<_> = msgs.iter().map(|m| m.as_slice()).collect();
                        let sigs: Vec<_> = sigs.iter().map(|s| s.as_slice()).collect();
                        let vks: Vec<_> = vks.iter().map(|k| k.as_slice()).collect();
                        assert!(ed25519_batch_verify(&msgs, &sigs, &vks).is_ok());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_hashers, bench_verifiers);

criterion_main!(benches);
