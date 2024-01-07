use {
    crate::Identity256,
    k256::ecdsa::{signature::DigestVerifier, Signature, VerifyingKey},
};

/// NOTE: This function takes the BLAKE3 hash of the message.
pub fn secp256k1_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> anyhow::Result<()> {
    let msg = Identity256::from_bytes(msg_hash)?;
    let sig = Signature::from_bytes(sig.into())?;
    let vk = VerifyingKey::from_sec1_bytes(pk)?;
    vk.verify_digest(msg, &sig).map_err(Into::into)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::identity_digest::hash,
        k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
        rand::rngs::OsRng,
    };

    #[test]
    fn verifying_secp256k1() {
        // generate a valid signature
        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let prehash_msg = b"Jake";
        let msg = hash(prehash_msg);
        let sig: Signature = sk.sign_digest(msg.clone());

        // valid signature
        {
            assert!(secp256k1_verify(
                msg.as_bytes(),
                sig.to_vec().as_slice(),
                vk.to_encoded_point(true).as_bytes()
            )
            .is_ok());
        }

        // incorrect private key
        {
            let false_sk = SigningKey::random(&mut OsRng);
            let false_sig: Signature = false_sk.sign_digest(msg.clone());
            assert!(secp256k1_verify(
                msg.as_bytes(),
                false_sig.to_vec().as_slice(),
                vk.to_encoded_point(true).as_bytes()
            )
            .is_err());
        }

        // incorrect public key
        {
            let false_sk = SigningKey::random(&mut OsRng);
            let false_vk = VerifyingKey::from(&false_sk);
            assert!(secp256k1_verify(
                msg.as_bytes(),
                sig.to_vec().as_slice(),
                false_vk.to_encoded_point(true).as_bytes()
            )
            .is_err());
        }

        // incorrect message
        {
            let false_prehash_msg = b"Larry";
            let false_msg = hash(false_prehash_msg);
            assert!(secp256k1_verify(
                false_msg.as_bytes(),
                sig.to_vec().as_slice(),
                vk.to_encoded_point(true).as_bytes()
            )
            .is_err());
        }
    }
}
