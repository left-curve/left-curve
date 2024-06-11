use {
    crate::{functions::to_sized, CryptoResult},
    ed25519_dalek::{Signature, VerifyingKey},
};

const ED25519_PUBKEY_LEN: usize = 32;
const ED25519_SIGNATURE_LEN: usize = 64;

/// Verify an ED25519 signature with the given hashed message and public
/// key.
///
/// NOTE: This function takes the hash of the message, not the prehash.
pub fn ed25519_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> CryptoResult<()> {
    // Validation
    let sig = to_sized::<ED25519_SIGNATURE_LEN>(sig)?;
    let pk = to_sized::<ED25519_PUBKEY_LEN>(pk)?;

    let vk = VerifyingKey::from_bytes(&pk)?;

    vk.verify_strict(msg_hash, &Signature::from(sig))
        .map_err(Into::into)
}

/// Verify a batch of ED25519 signatures with the given hashed message and public
/// key.
///
/// NOTE: This function takes the hash of the messages, not the prehash.
pub fn ed25519_batch_verify(
    msgs_hash: &[&[u8]],
    sigs: &[&[u8]],
    pks: &[&[u8]],
) -> CryptoResult<()> {
    let (sigs, vks): (Vec<_>, Vec<_>) = sigs
        .into_iter()
        .zip(pks.into_iter())
        .map(|(sig, pk)| {
            let sig = to_sized::<ED25519_SIGNATURE_LEN>(sig)?;
            let pk = to_sized::<ED25519_PUBKEY_LEN>(pk)?;
            Ok((Signature::from_bytes(&sig), VerifyingKey::from_bytes(&pk)?))
        })
        .collect::<CryptoResult<Vec<_>>>()?
        .into_iter()
        .unzip();

    // ed25519_dalek::verify_batch alredy check the length of messages, signatures and verifying_keys
    ed25519_dalek::verify_batch(msgs_hash, &sigs, &vks).map_err(Into::into)
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::identity_digest::hash,
        ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey},
        rand::rngs::OsRng,
    };

    #[test]
    fn verify_ed25519() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let prehash_msg = b"Jake";
        // let msg = hash(prehash_msg);
        let msg = hash(prehash_msg).as_bytes().to_vec();
        let sig = sk.sign(&msg);

        // valid signature
        assert!(ed25519_verify(&msg, sig.to_vec().as_slice(), vk.as_bytes()).is_ok());

        // incorrect private key
        {
            let false_sk = SigningKey::generate(&mut OsRng);
            let false_sig: Signature = false_sk.sign(&msg);
            assert!(ed25519_verify(&msg, false_sig.to_vec().as_slice(), vk.as_bytes()).is_err());
        }

        // incorrect message
        {
            let false_prehash_msg = b"Larry";
            let false_msg = hash(false_prehash_msg);
            assert!(
                ed25519_verify(false_msg.as_bytes(), sig.to_vec().as_slice(), vk.as_bytes())
                    .is_err()
            );
        }
    }

    #[test]
    fn verify_batch_ed25519() {
        macro_rules! join {
            ($($vec:expr),*) => {
                {
                    vec![$($vec.as_slice()),*]
                }
            };
        }

        let clos = |msg: &str| {
            let sk = SigningKey::generate(&mut OsRng);
            let vk = VerifyingKey::from(&sk);
            let prehash_msg = msg.as_bytes();
            let msg = hash(prehash_msg).as_bytes().to_vec();
            let sig = sk.sign(&msg);
            (msg, sig.to_bytes(), vk.to_bytes())
        };

        let (msg1, sig1, vk1) = clos("Jake");
        let (msg2, sig2, vk2) = clos("Larry");
        let (msg3, sig3, vk3) = clos("Rhaki");

        // valid signatures
        assert!(ed25519_batch_verify(
            &join!(msg1, msg2, msg3),
            &join!(sig1, sig2, sig3),
            &join!(vk1, vk2, vk3)
        )
        .is_ok());

        // wrong sign
        assert!(ed25519_batch_verify(
            &join!(msg1, msg2, msg3),
            &join!(sig2, sig1, sig3),
            &join!(vk1, vk2, vk3)
        )
        .is_err());

        // wrong len
        assert!(ed25519_batch_verify(
            &join!(msg1, msg2, msg3),
            &join!(sig1, sig2, sig3),
            &join!(vk1, vk2)
        )
        .is_err())
    }
}
