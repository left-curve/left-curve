use {
    ed25519_dalek::Signer,
    grug_crypto::{
        ed25519_verify, secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256,
        Identity256,
    },
    p256::ecdsa::signature::DigestSigner,
    rand::{rngs::OsRng, RngCore},
    std::time::Duration,
    test_case::test_case,
};

// cargo test --release --package grug-crypto --test benchmark -- benchmark --show-output
#[test_case(|msg: &[u8]| -> Duration {
    let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let (sig, _) = sk.sign_digest_recoverable(msg.clone()).unwrap();
    let now = std::time::Instant::now();
    secp256k1_verify(msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).unwrap();
    now.elapsed()};
"benchmark_secp256k1_verify")]
#[test_case(|msg: &[u8]| -> Duration {
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
#[test_case(|msg: &[u8]| -> Duration {
    let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
    let vk = p256::ecdsa::VerifyingKey::from(&sk);
    let msg = Identity256::from(sha2_256(msg));
    let sig: p256::ecdsa::Signature = sk.sign_digest(msg.clone());
    let now = std::time::Instant::now();
    secp256r1_verify(msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).unwrap();
    now.elapsed()};
"benchmark_secp256r1_verify")]
#[test_case(|msg: &[u8]| -> Duration {
    let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let vk = ed25519_dalek::VerifyingKey::from(&sk);
    let msg = sha2_256(msg);
    let sig = sk.sign(&msg);
    let now = std::time::Instant::now();
    ed25519_verify(&msg, &sig.to_bytes(), vk.as_bytes()).unwrap();
    now.elapsed()};
"benchmark_ed25519_verify")]
fn benchmark<FN: Fn(&[u8]) -> Duration>(clos: FN) {
    let mut tot_time = Duration::new(0, 0);
    let mut sum_log_time = 0.0;
    let iter = 100u32;
    for i in 1..iter + 1 {
        // Why not
        let i = i * 10;
        let mut vec = vec![0; i as usize];
        OsRng.fill_bytes(&mut vec);
        let time = clos(&vec);
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
