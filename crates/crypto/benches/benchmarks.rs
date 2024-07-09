use {
    criterion::{
        black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
        PlotConfiguration,
    },
    ed25519_dalek::Signer,
    grug_crypto::{
        blake2b_512, blake2s_256, blake3, ed25519_batch_verify, ed25519_verify, keccak256,
        secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256, sha2_512,
        sha2_512_truncated, sha3_256, sha3_512, sha3_512_truncated, Identity256,
    },
    p256::ecdsa::signature::DigestSigner,
    paste::paste,
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

macro_rules! bench {
    ($fn_name:ident, $settings:expr, $build_data:expr, $execute_data:expr) => {
        paste! {
            fn [<bench_$fn_name>](c: &mut Criterion) {
                let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Linear);
                let mut group = c.benchmark_group(stringify!{$fn_name});
                group.plot_config(plot_config);
                group.warm_up_time($settings.warmup_time);
                group.measurement_time($settings.measurement_time);
                for size in 1..$settings.iter + 1 {
                    let size = size * $settings.mul_iter;
                    let data = $build_data(size);
                    group.throughput(criterion::Throughput::Elements(size as u64));
                    group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
                        b.iter(|| $execute_data(black_box(&data)));
                    });
                }
                group.finish();
            }
        }
    };
}

// ---------------------------------- hashers ----------------------------------

bench!(sha2_256, HASH_SETTINGS, generate_random_msg, sha2_256);

bench!(sha2_512, HASH_SETTINGS, generate_random_msg, sha2_512);

bench!(
    sha2_512_truncated,
    HASH_SETTINGS,
    generate_random_msg,
    sha2_512_truncated
);

bench!(sha3_256, HASH_SETTINGS, generate_random_msg, sha3_256);

bench!(sha3_512, HASH_SETTINGS, generate_random_msg, sha3_512);

bench!(
    sha3_512_truncated,
    HASH_SETTINGS,
    generate_random_msg,
    sha3_512_truncated
);

bench!(keccak256, HASH_SETTINGS, generate_random_msg, keccak256);

bench!(blake2s_256, HASH_SETTINGS, generate_random_msg, blake2s_256);

bench!(blake2b_512, HASH_SETTINGS, generate_random_msg, blake2b_512);

bench!(blake3, HASH_SETTINGS, generate_random_msg, blake3);

// --------------------------------- verifiers ---------------------------------

fn secp256k1_verify_build(i: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let msg = &generate_random_msg(i);
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let (sig, _) = sk.sign_digest_recoverable(msg.clone()).unwrap();
    (
        msg.as_bytes().to_vec(),
        sig.to_bytes().to_vec(),
        vk.to_sec1_bytes().to_vec(),
    )
}

fn secp256k1_verify_execute((msg, sig, vk): &(Vec<u8>, Vec<u8>, Vec<u8>)) {
    secp256k1_verify(msg, sig, vk).unwrap();
}

fn secp256k1_pubkey_recover_build(i: usize) -> (Vec<u8>, Vec<u8>, u8, bool) {
    let msg = &generate_random_msg(i);
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let msg = Identity256::from(sha2_256(msg));
    let (sig, recovery_id) = sk.sign_digest_recoverable(msg.clone()).unwrap();

    (
        msg.as_bytes().to_vec(),
        sig.to_bytes().to_vec(),
        recovery_id.to_byte(),
        false,
    )
}

fn secp256k1_pubkey_recover_execute(
    (msg, sig, recover_id, compressed): &(Vec<u8>, Vec<u8>, u8, bool),
) {
    secp256k1_pubkey_recover(msg, sig, *recover_id, *compressed).unwrap();
}

fn secp256r1_verify_build(i: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let msg = &generate_random_msg(i);
    let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = p256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let sig: p256::ecdsa::Signature = sk.sign_digest(msg.clone());
    (
        msg.as_bytes().to_vec(),
        sig.to_bytes().to_vec(),
        vk.to_sec1_bytes().to_vec(),
    )
}

fn secp256r1_verify_execute((msg, sig, vk): &(Vec<u8>, Vec<u8>, Vec<u8>)) {
    secp256r1_verify(msg, sig, vk).unwrap();
}

fn ed25519_verify_build(i: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let msg = &generate_random_msg(i);
    let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let vk = ed25519_dalek::VerifyingKey::from(&sk);
    let msg = sha2_256(msg);
    let sig = sk.sign(&msg);
    (
        msg.to_vec(),
        sig.to_bytes().to_vec(),
        vk.as_bytes().to_vec(),
    )
}

fn ed25519_verify_execute((msg, sig, vk): &(Vec<u8>, Vec<u8>, Vec<u8>)) {
    ed25519_verify(msg, sig, vk).unwrap()
}

fn ed25519_verify_batch_build(i: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut msgs: Vec<Vec<u8>> = vec![];
    let mut sigs: Vec<Vec<u8>> = vec![];
    let mut vks: Vec<Vec<u8>> = vec![];

    for _ in 1..i + 1 {
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let vk = ed25519_dalek::VerifyingKey::from(&sk);
        let msg = sha2_256(&generate_random_msg(i));
        let sig = sk.sign(&msg);
        msgs.push(msg.to_vec());
        sigs.push(sig.to_bytes().to_vec());
        vks.push(vk.to_bytes().to_vec());
    }
    (msgs, sigs, vks)
}

fn ed25519_verify_batch_execute((msgs, sigs, vks): &(Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)) {
    let msgs = msgs.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    let sigs = sigs.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    let vks = vks.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    ed25519_batch_verify(&msgs, &sigs, &vks).unwrap()
}

bench!(
    secp256k1_verify,
    CRYPTO_SETTINGS,
    secp256k1_verify_build,
    secp256k1_verify_execute
);

bench!(
    secp256k1_pubkey_recover,
    CRYPTO_SETTINGS,
    secp256k1_pubkey_recover_build,
    secp256k1_pubkey_recover_execute
);

bench!(
    secp256r1_verify,
    CRYPTO_SETTINGS,
    secp256r1_verify_build,
    secp256r1_verify_execute
);

bench!(
    ed25519_verify,
    CRYPTO_SETTINGS,
    ed25519_verify_build,
    ed25519_verify_execute
);

bench!(
    ed25519_verify_batch,
    CRYPTO_SETTINGS,
    ed25519_verify_batch_build,
    ed25519_verify_batch_execute
);

// ----------------------------------- main ------------------------------------

criterion_group!(
    benches,
    bench_sha2_256,
    bench_sha2_512,
    bench_sha2_512_truncated,
    bench_sha3_256,
    bench_sha3_512,
    bench_sha3_512_truncated,
    bench_keccak256,
    bench_blake2s_256,
    bench_blake2b_512,
    bench_blake3,
    bench_secp256k1_verify,
    bench_secp256k1_pubkey_recover,
    bench_secp256r1_verify,
    bench_ed25519_verify,
    bench_ed25519_verify_batch
);

criterion_main!(benches);
