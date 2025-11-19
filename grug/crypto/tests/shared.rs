use {
    grug_crypto::secp256k1_pubkey_recover,
    serde::de::DeserializeOwned,
    std::{fs::File, io::BufReader},
};

#[allow(clippy::unwrap_used, reason = "this code is only used in tests")]
pub fn read_file<F>(path: &str) -> F
where
    F: DeserializeOwned,
{
    // Open the file in read-only mode with buffer.
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).unwrap()
}

#[allow(clippy::unwrap_used, reason = "this code is only used in tests")]
pub fn validate_recover_secp256k1(
    message_hash: &[u8],
    signature: &[u8],
    public_key: &[u8],
    params: [u8; 2],
    compressed: bool,
) {
    // Since the recovery param is missing in the test vectors, we try both
    let recovered0 =
        secp256k1_pubkey_recover(message_hash, signature, params[0], compressed).unwrap();
    let recovered1 =
        secp256k1_pubkey_recover(message_hash, signature, params[1], compressed).unwrap();
    // Got two different pubkeys. Without the recovery param, we don't know which one is the right one.
    assert_ne!(recovered0, recovered1);
    assert!(recovered0 == public_key || recovered1 == public_key);
}

pub fn validate_recover_secp256r1(
    _message_hash: &[u8],
    _signature: &[u8],
    _public_key: &[u8],
    _params: [u8; 2],
    _compressed: bool,
) {
    // We don't have r1 recover, so just mock it
}
