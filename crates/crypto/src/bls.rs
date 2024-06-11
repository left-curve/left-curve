use bls_signatures::{PublicKey, Serialize, Signature};

use crate::CryptoResult;

/// NOTE: This function takes the hash of the message, not the prehash.
pub fn bls_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> CryptoResult<()> {
    let pk = PublicKey::from_bytes(pk)?;

    if pk.verify(Signature::from_bytes(sig)?, msg_hash) {
        Ok(())
    } else {
        Err(signature::Error::new().into())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        bls_signatures::{PrivateKey, Serialize},
        rand::rngs::OsRng,
    };

    use crate::{bls::bls_verify, identity_digest::hash};

    #[test]
    fn verifying_bls() {
        let sk = PrivateKey::generate(&mut OsRng);

        let pk = sk.public_key();

        let prehash_msg = b"Jake";
        let msg = hash(prehash_msg);
        let msg = msg.as_bytes().to_vec();

        let sig = sk.sign(&msg);

        // valid signature
        assert!(bls_verify(&msg, &sig.as_bytes(), &pk.as_bytes()).is_ok());

        // incorrect private key
        {
            let false_sk = PrivateKey::generate(&mut OsRng);
            let false_sig = false_sk.sign(msg.clone());
            assert!(bls_verify(&msg, &false_sig.as_bytes(), &pk.as_bytes()).is_err());
        }

        // incorrect public key
        {
            let false_sk = PrivateKey::generate(&mut OsRng);
            assert!(bls_verify(&msg, &sig.as_bytes(), &false_sk.public_key().as_bytes()).is_err());
        }
    }
}
