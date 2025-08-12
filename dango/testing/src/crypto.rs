use {
    digest::{consts::U32, generic_array::GenericArray},
    grug::ByteArray,
    identity::Identity256,
    k256::{
        ecdsa::{Signature, SigningKey, signature::DigestSigner},
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

/// Note: This function expects the _hashed_ sign data.
pub fn create_signature(sk: &SigningKey, sign_data: GenericArray<u8, U32>) -> ByteArray<64> {
    let sign_data = Identity256::from_inner(sign_data);
    let signature: Signature = sk.sign_digest(sign_data);

    ByteArray::from_inner(signature.to_bytes().into())
}
