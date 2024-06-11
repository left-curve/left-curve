use {
    crate::{to_sized, CryptoError, CryptoResult, Identity256},
    k256::ecdsa::{signature::DigestVerifier, RecoveryId, Signature, VerifyingKey},
};

const SECP256K1_PUBKEY_LEN: usize = 33;
const SECP256K1_SIGNATURE_LEN: usize = 64;

/// NOTE: This function takes the hash of the message, not the prehash.
pub fn secp256k1_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> CryptoResult<()> {
    let msg = Identity256::from_slice(msg_hash)?;

    let sig = to_sized::<SECP256K1_SIGNATURE_LEN>(sig)?;
    let sig = Signature::from_bytes(&sig.into())?;

    let vk = to_sized::<SECP256K1_PUBKEY_LEN>(pk)?;
    let vk = VerifyingKey::from_sec1_bytes(&vk)?;

    vk.verify_digest(msg, &sig).map_err(Into::into)
}

/// Recover the Secp256k1 public key as SEC1 bytes from the _hashed_ message and
/// signature.
///
/// - `r`: the first 32 bytes of the signature;
/// - `s`: the last 32 bytes of the signature;
/// - `v`: the recovery ID.
///
/// `v` must be 0 or 1. The values 2 and 3 are unsupported by this implementation,
/// which is the same restriction [as Ethereum has](https://github.com/ethereum/go-ethereum/blob/v1.9.25/internal/ethapi/api.go#L466-L469).
/// All other values are invalid.
///
/// Note: This function takes the hash of the message, not the prehash.
pub fn secp256k1_pubkey_recover(
    msg_hash: &[u8],
    sig: &[u8],
    recovery_id: u8,
) -> CryptoResult<Vec<u8>> {
    let msg = Identity256::from_slice(msg_hash)?;

    let sig = to_sized::<SECP256K1_SIGNATURE_LEN>(sig)?;
    let mut sig = Signature::from_bytes(&sig.into())?;

    let mut id = match recovery_id {
        0 => RecoveryId::new(false, false),
        1 => RecoveryId::new(true, false),
        _ => return Err(CryptoError::InvalidRecoveryId { recovery_id }),
    };

    if let Some(normalized) = sig.normalize_s() {
        sig = normalized;
        id = RecoveryId::new(!id.is_y_odd(), id.is_x_reduced());
    }

    // Convert the public key to SEC1 bytes
    VerifyingKey::recover_from_digest(msg, &sig, id)
        .map(|vk| vk.to_sec1_bytes().to_vec())
        .map_err(Into::into)
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
        assert!(secp256k1_verify(msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).is_ok());

        // incorrect private key
        let false_sk = SigningKey::random(&mut OsRng);
        let false_sig: Signature = false_sk.sign_digest(msg.clone());
        assert!(
            secp256k1_verify(msg.as_bytes(), &false_sig.to_bytes(), &vk.to_sec1_bytes()).is_err()
        );

        // incorrect public key
        let false_sk = SigningKey::random(&mut OsRng);
        let false_vk = VerifyingKey::from(&false_sk);
        assert!(
            secp256k1_verify(msg.as_bytes(), &sig.to_bytes(), &false_vk.to_sec1_bytes()).is_err()
        );

        // incorrect message
        let false_prehash_msg = b"Larry";
        let false_msg = hash(false_prehash_msg);
        assert!(
            secp256k1_verify(false_msg.as_bytes(), &sig.to_bytes(), &vk.to_sec1_bytes()).is_err()
        );
    }

    #[test]
    fn recovering_secp256k1() {
        // generate a valid signature
        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let prehash_msg = b"Jake";
        let msg = hash(prehash_msg);
        let (sig, recovery_id) = sk.sign_digest_recoverable(msg.clone()).unwrap();

        let recovered_pk = secp256k1_pubkey_recover(
            msg.as_bytes(),
            sig.to_vec().as_slice(),
            recovery_id.to_byte(),
        )
        .unwrap();
        assert_eq!(recovered_pk, vk.to_sec1_bytes().to_vec());
    }
}
