use {
    crate::{functions::to_sized, CryptoError, CryptoResult, Identity256},
    k256::ecdsa::{signature::DigestVerifier, RecoveryId, Signature, VerifyingKey},
};

/// NOTE: This function takes the hash of the message, not the prehash.
pub fn secp256k1_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> CryptoResult<()> {
    let msg = Identity256::from_slice(msg_hash)?;
    // NOTE: sig.into() here will panic if the byte slice is of incorrect length,
    // crashing the node. we must safe guard this
    to_sized::<64>(sig)?;
    let sig = Signature::from_bytes(sig.into())?;
    let vk = VerifyingKey::from_sec1_bytes(pk)?;
    vk.verify_digest(msg, &sig).map_err(Into::into)
}

/// Recover the compressed byte of the `public key` from the `signature` and `message hash`.
/// - **r**: the first `32 bytes` of the signature;
/// - **s**: the last `32 bytes` of the signature;
/// - **v**: the `recovery id`.
///
/// `v` must be 0 or 1. The values 2 and 3 are unsupported by this implementation,
/// which is the same restriction as Ethereum has (https://github.com/ethereum/go-ethereum/blob/v1.9.25/internal/ethapi/api.go#L466-L469).
/// All other values are invalid.
/// Note: this function takes the hash of the message, not the prehash.
pub fn secp256k1_pubkey_recover(
    msg_hash: &[u8],
    r: &[u8],
    s: &[u8],
    v: u8,
) -> CryptoResult<Vec<u8>> {
    let msg_hash = to_sized::<32>(msg_hash)?;
    let r = to_sized::<32>(r)?;
    let s = to_sized::<32>(s)?;

    let mut id = match v {
        0 => RecoveryId::new(false, false),
        1 => RecoveryId::new(true, false),
        _ => return Err(CryptoError::invalid_recovery_id(v)),
    };

    let mut sig = Signature::from_scalars(r, s)?;

    let message_digest = Identity256::from_slice(&msg_hash)?;

    if let Some(normalized) = sig.normalize_s() {
        sig = normalized;
        id = RecoveryId::new(!id.is_y_odd(), id.is_x_reduced());
    }

    let pubkey = VerifyingKey::recover_from_digest(message_digest, &sig, id)?;
    Ok(pubkey.to_encoded_point(true).as_bytes().into())
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

    #[test]
    fn recovering_secp256k1() {
        // generate a valid signature
        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let prehash_msg = b"Jake";
        let msg = hash(prehash_msg);
        let (sig, recover) = sk.sign_digest_recoverable(msg.clone()).unwrap();

        let sig = sig.to_bytes().to_vec();
        let (r, s) = sig.split_at(sig.len() / 2);

        // recover pub key
        {
            let recovered_pk =
                secp256k1_pubkey_recover(msg.as_bytes(), r, s, recover.to_byte()).unwrap();
            assert_eq!(recovered_pk, vk.to_encoded_point(true).as_bytes());
        }
    }
}
