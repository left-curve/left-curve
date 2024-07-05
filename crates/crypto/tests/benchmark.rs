use {
    ed25519_dalek::Signer,
    grug_crypto::{
        blake2b_512, blake2s_256, blake3, ed25519_batch_verify, ed25519_verify, keccak256,
        secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256, sha2_512,
        sha2_512_truncated, sha3_256, sha3_512, sha3_512_truncated, Identity256,
    },
    p256::ecdsa::signature::DigestSigner,
    rand::{rngs::OsRng, RngCore},
    std::time::Duration,
    test_case::test_case,
};

fn gen_msg(i: usize) -> Vec<u8> {
    let mut vec = vec![0; i];
    OsRng.fill_bytes(&mut vec);
    vec
}

// cargo test --release --package grug-crypto --test benchmark -- benchmark --show-output

#[test_case(|i: usize| -> Duration {
    let msg = &gen_msg(i);
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let (sig, _) = sk.sign_digest_recoverable(msg.clone()).unwrap();
    let now = std::time::Instant::now();
    secp256k1_verify(msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).unwrap();
    now.elapsed()};
"benchmark_secp256k1_verify")]
#[test_case(|i: usize| -> Duration {
    let msg = &gen_msg(i);
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let msg = Identity256::from(sha2_256(msg));
    let (sig, recovery_id) = sk.sign_digest_recoverable(msg.clone()).unwrap();
    let now = std::time::Instant::now();
    secp256k1_pubkey_recover(
        msg.as_bytes(),
        sig.to_vec().as_slice(),
        recovery_id.to_byte(),
        true,
    )
    .unwrap();
    now.elapsed()};
"benchmark_secp256k1_pubkey_recover")]
#[test_case(|i: usize| -> Duration {
    let msg = &gen_msg(i);
    let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = p256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let sig: p256::ecdsa::Signature = sk.sign_digest(msg.clone());
    let now = std::time::Instant::now();
    secp256r1_verify(msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).unwrap();
    now.elapsed()};
"benchmark_secp256r1_verify")]
#[test_case(|i: usize| -> Duration {
    let msg = &gen_msg(i);
    let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let vk = ed25519_dalek::VerifyingKey::from(&sk);
    let msg = sha2_256(msg);
    let sig = sk.sign(&msg);
    let now = std::time::Instant::now();
    ed25519_verify(&msg, &sig.to_bytes(), vk.as_bytes()).unwrap();
    now.elapsed()};
"benchmark_ed25519_verify")]
fn benchmark<FN: Fn(usize) -> Duration>(clos: FN) {
    let mut tot_time = Duration::new(0, 0);
    let mut sum_log_time = 0.0;
    let iter = 100u32;
    for i in 1..iter + 1 {
        // Why not
        let i = i * 10;
        let mut vec = vec![0; i as usize];
        OsRng.fill_bytes(&mut vec);
        let time = clos(i as usize);
        tot_time += time;
        sum_log_time += (time.as_nanos() as f64).ln();
    }
    let ari_mean = tot_time / (iter);
    let geo_mean = (sum_log_time / (iter) as f64).exp();
    let geo_mean = Duration::from_nanos(geo_mean as u64);
    println!(
        "Arithmetic mean: {:?} - Geometric mean: {:?} - iterations: {}",
        ari_mean, geo_mean, iter
    );
}

// cargo test --release --package grug-crypto --test benchmark -- linear_benchmark --show-output

#[test_case(1, |i: usize| {
    let mut msgs: Vec<Vec<u8>> = vec![];
    let mut sigs: Vec<Vec<u8>> = vec![];
    let mut vks: Vec<Vec<u8>> = vec![];

    for _ in 1..i + 1 {
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let vk = ed25519_dalek::VerifyingKey::from(&sk);
        let msg = sha2_256(&gen_msg(i));
        let sig = sk.sign(&msg);
        msgs.push(msg.to_vec());
        sigs.push(sig.to_bytes().to_vec());
        vks.push(vk.to_bytes().to_vec());
    }

    let msgs = msgs.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    let sigs = sigs.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
    let vks = vks.iter().map(|x| x.as_slice()).collect::<Vec<_>>();

    let now = std::time::Instant::now();
    ed25519_batch_verify(&msgs, &sigs, &vks).unwrap();
    now.elapsed()};
"benchmark_ed25519_verify_batch")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha2_256(&gen_msg(i));
    now.elapsed()};
"benchmark_sha_256")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha2_512(&gen_msg(i));
    now.elapsed()};
"benchmark_sha_512")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha2_512_truncated(&gen_msg(i));
    now.elapsed()};
"benchmark_sha_512_truncated")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha3_256(&gen_msg(i));
    now.elapsed()};
"benchmark_sha3_256")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha3_512(&gen_msg(i));
    now.elapsed()};
"benchmark_sha3_512")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    sha3_512_truncated(&gen_msg(i));
    now.elapsed()};
"benchmark_sha3_512_truncated")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    keccak256(&gen_msg(i));
    now.elapsed()};
"benchmark_keccak256")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    blake2s_256(&gen_msg(i));
    now.elapsed()};
"benchmark_blake2s_256")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    blake2b_512(&gen_msg(i));
    now.elapsed()};
"benchmark_blake2b_512")]
#[test_case(100, |i: usize| {
    let now = std::time::Instant::now();
    blake3(&gen_msg(i));
    now.elapsed()};
"benchmark_blake3")]
fn linear_benchmark<FN: Fn(usize) -> Duration>(mul: u32, clos: FN) {
    let mut tot_time = Duration::new(0, 0);
    let mut sum_log_time = 0.0;
    let iter = 100u32;

    let mut last_iter: Option<Duration> = None;
    let mut tot_base = Duration::new(0, 0);
    let mut tot_per_item = Duration::new(0, 0);
    let mut linear_counter = 0;
    for i in 1..iter + 1 {
        // Why not
        let i = i * mul;
        let time = clos(i as usize);
        tot_time += time;
        sum_log_time += (time.as_nanos() as f64).ln();

        if let Some(pre_time) = &mut last_iter {
            let dif = match time.checked_sub(*pre_time) {
                Some(dif) => dif / mul,
                None => {
                    *pre_time = time;
                    continue;
                },
            };

            let items = dif * i;
            let base = time.checked_sub(items).unwrap_or_default();
            tot_base += base;
            tot_per_item += items / i;
            linear_counter += 1;
            *pre_time = time;
        } else {
            last_iter = Some(time);
        }
    }
    let ari_mean = tot_time / (iter);
    let geo_mean = (sum_log_time / (iter) as f64).exp();
    let geo_mean = Duration::from_nanos(geo_mean as u64);

    let avg_linear = tot_base / linear_counter;
    let avg_non_linear = tot_per_item / linear_counter;
    println!(
        "Arithmetic mean: {:?} - Geometric mean: {:?} - iterations: {}",
        ari_mean, geo_mean, iter
    );

    println!(
        "Arithmetic mean base: {:?} - Arithmetic mean per_item: {:?} - valid_iterations: {}",
        avg_linear, avg_non_linear, linear_counter
    );
}
