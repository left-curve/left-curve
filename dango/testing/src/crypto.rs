use {
    grug::ByteArray,
    k256::{
        ecdsa::{Signature, SigningKey, signature::Signer},
        elliptic_curve::rand_core::OsRng,
    },
};

pub fn generate_random_key() -> (SigningKey, ByteArray<33>) {
    let sk = SigningKey::random(&mut OsRng);
    let pk = sk
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes()
        .as_ref()
        .try_into()
        .unwrap();

    (sk, pk)
}

/// Note: This function expects the _prehash_ bytes.
pub fn create_signature(sk: &SigningKey, prehash_sign_data: &[u8]) -> ByteArray<64> {
    // This hashes `prehash_sign_data` with SHA2-256. If we eventually choose to
    // use a different hash, it's necessary to update this.
    let signature: Signature = sk.sign(prehash_sign_data);

    signature.to_bytes().as_slice().try_into().unwrap()
}
