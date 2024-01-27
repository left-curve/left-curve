use {
    bip32::{Mnemonic, XPrv},
    cw_crypto::Identity256,
    k256::ecdsa::{Signature, VerifyingKey},
    signature::{DigestSigner, Signer},
};

/// A wrapper over k256 SigningKey, providing a handy API to work with.
pub struct SigningKey {
    pub(crate) inner: k256::ecdsa::SigningKey,
}

impl SigningKey {
    /// Note: Only support secp256k1, not r1. This is because we use Bitcoin's
    /// BIP-32 library, and Bitcoin only uses k1.
    pub fn derive_from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self> {
        // The `to_seed` function takes a password to generate salt.
        // Here we just use an empty str.
        // For reference, Terra Station and Keplr use an empty string as well:
        // - https://github.com/terra-money/terra.js/blob/v3.1.7/src/key/MnemonicKey.ts#L79
        // - https://github.com/chainapsis/keplr-wallet/blob/b6062a4d24f3dcb15dda063b1ece7d1fbffdbfc8/packages/crypto/src/mnemonic.ts#L63
        let seed = mnemonic.to_seed("");
        let path = format!("m/44'/{coin_type}'/0'/0/0");
        let xprv = XPrv::derive_from_path(&seed, &path.parse()?)?;
        Ok(Self {
            inner: xprv.into(),
        })
    }

    pub fn sign_bytes(&self, bytes: &[u8]) -> Signature {
        self.inner.sign(bytes)
    }

    pub fn sign_digest(&self, digest: &[u8; 32]) -> Signature {
        let digest = Identity256::from_bytes(digest);
        self.inner.sign_digest(digest)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes().into()
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        self.inner.verifying_key()
    }
}
