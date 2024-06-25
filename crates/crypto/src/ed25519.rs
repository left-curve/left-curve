use {
    crate::{to_sized, CryptoResult},
    ed25519_dalek::{Signature, VerifyingKey},
};

const ED25519_PUBKEY_LEN: usize = 32;
const ED25519_SIGNATURE_LEN: usize = 64;

/// Verify an ED25519 signature with the given hashed message and public
/// key.
///
/// NOTE: This function takes the hash of the message, not the prehash.
pub fn ed25519_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> CryptoResult<()> {
    let sig = to_sized::<ED25519_SIGNATURE_LEN>(sig)?;
    let sig = Signature::from(sig);

    let vk = to_sized::<ED25519_PUBKEY_LEN>(pk)?;
    let vk = VerifyingKey::from_bytes(&vk)?;

    vk.verify_strict(msg_hash, &sig).map_err(Into::into)
}

/// Verify a batch of ED25519 signatures with the given hashed message and public
/// key.
///
/// NOTE: This function takes the hash of the messages, not the prehash.
pub fn ed25519_verify_batch(
    msgs_hash: &[&[u8]],
    sigs: &[&[u8]],
    pks: &[&[u8]],
) -> CryptoResult<()> {
    let (sigs, vks): (Vec<_>, Vec<_>) = sigs
        .iter()
        .zip(pks.iter())
        .map(|(sig, pk)| {
            let sig = to_sized::<ED25519_SIGNATURE_LEN>(sig)?;
            let sig = Signature::from(sig);

            let vk = to_sized::<ED25519_PUBKEY_LEN>(pk)?;
            let vk = VerifyingKey::from_bytes(&vk)?;

            Ok((sig, vk))
        })
        .collect::<CryptoResult<Vec<_>>>()?
        .into_iter()
        .unzip();

    // No need to check the three slices (`msgs_hash`, `sigs`, `pks`) are of the
    // length; `ed25519_dalek::verify_batch` already does this.
    ed25519_dalek::verify_batch(msgs_hash, &sigs, &vks).map_err(Into::into)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{sha2_256, Identity256},
        ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey},
        rand::rngs::OsRng,
    };

    #[test]
    fn verify_ed25519() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let prehash_msg = b"Jake";
        let msg = sha2_256(prehash_msg);
        let sig = sk.sign(&msg);

        // valid signature
        assert!(ed25519_verify(&msg, &sig.to_bytes(), vk.as_bytes()).is_ok());

        // incorrect private key
        let false_sk = SigningKey::generate(&mut OsRng);
        let false_sig: Signature = false_sk.sign(&msg);
        assert!(ed25519_verify(&msg, &false_sig.to_bytes(), vk.as_bytes()).is_err());

        // incorrect message
        let false_prehash_msg = b"Larry";
        let false_msg = sha2_256(false_prehash_msg);
        assert!(ed25519_verify(&false_msg, &sig.to_bytes(), vk.as_bytes()).is_err());
    }

    #[test]
    fn verify_batch_ed25519() {
        let (msg1, sig1, vk1) = ed25519_sign("Jake");
        let (msg2, sig2, vk2) = ed25519_sign("Larry");
        let (msg3, sig3, vk3) = ed25519_sign("Rhaki");

        // valid signatures
        assert!(ed25519_verify_batch(
            &[msg1.as_bytes(), msg2.as_bytes(), msg3.as_bytes()],
            &[&sig1, &sig2, &sig3],
            &[&vk1, &vk2, &vk3]
        )
        .is_ok());

        // wrong sign
        assert!(ed25519_verify_batch(
            &[msg1.as_bytes(), msg2.as_bytes(), msg3.as_bytes()],
            &[&sig2, &sig1, &sig3],
            &[&vk1, &vk2, &vk3]
        )
        .is_err());

        // wrong len
        assert!(ed25519_verify_batch(
            &[msg1.as_bytes(), msg2.as_bytes(), msg3.as_bytes()],
            &[&sig1, &sig2, &sig3],
            &[&vk1, &vk2]
        )
        .is_err());
    }

    fn ed25519_sign(prehash_msg: &str) -> (Identity256, [u8; 64], [u8; 32]) {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let msg = Identity256::from(sha2_256(prehash_msg.as_bytes()));
        let sig = sk.sign(msg.as_bytes());
        (msg, sig.to_bytes(), vk.to_bytes())
    }
}
