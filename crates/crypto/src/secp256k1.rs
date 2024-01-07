use {
    crate::identity_digest::hash,
    k256::ecdsa::{signature::DigestVerifier, Signature, VerifyingKey},
};

pub fn secp256k1_verify(prehash_msg: &[u8], sig: &[u8], pk: &[u8]) -> anyhow::Result<()> {
    let msg = hash(prehash_msg);
    let sig = Signature::from_bytes(sig.into())?;
    let vk = VerifyingKey::from_sec1_bytes(pk)?;
    vk.verify_digest(msg, &sig).map_err(Into::into)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
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
            assert!(vk.verify_digest(msg.clone(), &sig).is_ok());
        }

        // incorrect private key
        {
            let false_sk = SigningKey::random(&mut OsRng);
            let false_sig: Signature = false_sk.sign_digest(msg.clone());
            assert!(vk.verify_digest(msg.clone(), &false_sig).is_err());
        }

        // incorrect public key
        {
            let false_sk = SigningKey::random(&mut OsRng);
            let false_vk = VerifyingKey::from(&false_sk);
            assert!(false_vk.verify_digest(msg, &sig).is_err());
        }

        // incorrect message
        {
            let false_prehash_msg = b"Larry";
            let false_msg = hash(false_prehash_msg);
            assert!(vk.verify_digest(false_msg, &sig).is_err());
        }
    }
}
