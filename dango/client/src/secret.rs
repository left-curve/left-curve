use {
    alloy::{
        dyn_abi::{Eip712Domain, TypedData},
        primitives::U160,
    },
    bip32::{Mnemonic, PublicKey, XPrv},
    dango_auth::EIP155_CHAIN_ID,
    dango_types::auth::{Eip712Signature, Key, SignDoc, Signature},
    grug::{ByteArray, Hash256, HashExt, Inner, JsonDeExt, JsonSerExt, SignData, json},
    identity::Identity256,
    k256::{ecdsa::signature::DigestSigner, schnorr::CryptoRngCore},
    rand::rngs::OsRng,
};

/// Represents a secret key that can sign transactions.
pub trait Secret: Sized {
    /// Byte array representing the private key.
    type Private;

    /// Byte array representing the public key associated with this private key.
    type Public;

    /// Byte array representing the signature produced by this secret key.
    type Signature;

    /// Generate a new random private key with the [`OsRng`](https://docs.rs/rand/latest/rand/rngs/struct.OsRng.html).
    fn new_random() -> Self {
        Self::from_rng(&mut OsRng)
    }

    /// Generate a new random private key with the given RNG.
    fn from_rng(rng: &mut impl CryptoRngCore) -> Self;

    /// Recover a private key from raw bytes.
    fn from_bytes(bytes: Self::Private) -> anyhow::Result<Self>;

    /// Recover a private key from the given English mnemonic and BIP-44 coin type.
    fn from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self>;

    /// Return the private key as a byte array.
    fn private_key(&self) -> Self::Private;

    /// Return the compressed public key as a byte array.
    fn public_key(&self) -> Self::Public;

    /// Return the [`Key`](dango_types::auth::Key) for using in the Dango account factory.
    fn key(&self) -> Key;

    /// Return the key hash for use in the Dango account factory.
    fn key_hash(&self) -> Hash256;

    /// Sign the given sign doc.
    fn sign_transaction(&self, sign_doc: SignDoc) -> anyhow::Result<Signature>;
}

// --------------------------------- Secp256r1 ---------------------------------

// TODO: Secp256r1 secret.

// --------------------------------- Secp256k1 ---------------------------------

/// An Secp256k1 private key.
pub struct Secp256k1 {
    inner: k256::ecdsa::SigningKey,
}

impl Secret for Secp256k1 {
    type Private = [u8; 32];
    type Public = [u8; 33];
    type Signature = [u8; 64];

    fn from_rng(rng: &mut impl CryptoRngCore) -> Self {
        Self {
            inner: k256::ecdsa::SigningKey::random(rng),
        }
    }

    fn from_bytes(bytes: [u8; 32]) -> anyhow::Result<Self> {
        Ok(Self {
            inner: k256::ecdsa::SigningKey::from_bytes(&bytes.into())?,
        })
    }

    fn from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self> {
        // The `to_seed` function takes a password to generate salt.
        // Here we just use an empty str.
        // For reference, Terra Station and Keplr use an empty string as well:
        // - https://github.com/terra-money/terra.js/blob/v3.1.7/src/key/MnemonicKey.ts#L79
        // - https://github.com/chainapsis/keplr-wallet/blob/b6062a4d24f3dcb15dda063b1ece7d1fbffdbfc8/packages/crypto/src/mnemonic.ts#L63
        let seed = mnemonic.to_seed("");
        let path = format!("m/44'/{coin_type}'/0'/0/0");
        let xprv = XPrv::derive_from_path(&seed, &path.parse()?)?;

        Ok(Self { inner: xprv.into() })
    }

    fn private_key(&self) -> [u8; 32] {
        self.inner.to_bytes().into()
    }

    fn public_key(&self) -> [u8; 33] {
        self.inner.verifying_key().to_bytes()
    }

    fn key(&self) -> Key {
        Key::Secp256k1(self.public_key().into())
    }

    fn key_hash(&self) -> Hash256 {
        self.public_key().hash256() // SHA-256 key
    }

    fn sign_transaction(&self, sign_doc: SignDoc) -> anyhow::Result<Signature> {
        let sign_data = sign_doc.to_sign_data()?;
        let digest = Identity256::from_inner(sign_data);
        let signature: k256::ecdsa::Signature = self.inner.sign_digest(digest);

        Ok(Signature::Secp256k1(ByteArray::from_inner(
            signature.to_bytes().into(),
        )))
    }
}

// --------------------------------- Ethereum ----------------------------------

/// An Secp256k1 private key that signs message in Ethereum EIP-712 format.
pub struct Eip712 {
    inner: Secp256k1,
    // This means the Ethereum address, not the Dango address.
    pub address: eth_utils::Address,
}

impl Secret for Eip712 {
    type Private = <Secp256k1 as Secret>::Private;
    type Public = <Secp256k1 as Secret>::Public;
    // 64 bytes of signature + 1 byte of recovery ID
    type Signature = [u8; 65];

    fn from_rng(rng: &mut impl CryptoRngCore) -> Self {
        Secp256k1::from_rng(rng).into()
    }

    fn from_bytes(bytes: [u8; 32]) -> anyhow::Result<Self> {
        Secp256k1::from_bytes(bytes).map(Into::into)
    }

    fn from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self> {
        Secp256k1::from_mnemonic(mnemonic, coin_type).map(Into::into)
    }

    fn private_key(&self) -> [u8; 32] {
        self.inner.private_key()
    }

    fn public_key(&self) -> [u8; 33] {
        self.inner.public_key()
    }

    fn key(&self) -> dango_types::auth::Key {
        dango_types::auth::Key::Ethereum(self.address.into())
    }

    fn key_hash(&self) -> Hash256 {
        self.address.hash256()
    }

    fn sign_transaction(&self, sign_doc: SignDoc) -> anyhow::Result<Signature> {
        // EIP-712 hash used in the signature.
        let data = TypedData {
            resolver: json!({"Message":[]}).deserialize_json()?,
            domain: Eip712Domain {
                name: Some("dango".into()),
                chain_id: Some(EIP155_CHAIN_ID),
                verifying_contract: Some(U160::from_be_bytes(sign_doc.sender.into_inner()).into()),
                ..Default::default()
            },
            primary_type: "Message".to_string(),
            message: sign_doc.to_json_value()?.into_inner(),
        };

        let sign_bytes = data.eip712_signing_hash()?;
        let digest = Identity256::from(sign_bytes.0);
        let (signature, recovery_id) = self.inner.inner.sign_digest_recoverable(digest)?;

        Ok(Signature::Eip712(Eip712Signature {
            typed_data: data.to_json_vec()?.into(),
            sig: eth_utils::pack_signature(signature, recovery_id).into(),
        }))
    }
}

impl From<Secp256k1> for Eip712 {
    fn from(inner: Secp256k1) -> Self {
        let address = eth_utils::derive_address(inner.inner.verifying_key());
        Self { inner, address }
    }
}
