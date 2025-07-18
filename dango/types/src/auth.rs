use {
    crate::account_factory::Username,
    grug::{
        Addr, Binary, ByteArray, Hash256, JsonSerExt, Message, NonEmpty, SignData, StdError,
        Timestamp,
    },
    serde::{Deserialize, Serialize},
    sha2::Sha256,
    std::fmt::Display,
};

/// A number that included in each transaction's sign doc for the purpose of
/// replay protection.
pub type Nonce = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
#[cfg_attr(
    feature = "sea-orm",
    derive(sea_orm::EnumIter, sea_orm::DeriveActiveEnum)
)]
#[cfg_attr(
    feature = "sea-orm",
    sea_orm(rs_type = "i16", db_type = "SmallInteger")
)]
pub enum KeyType {
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 0))]
    Secp256r1,
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 1))]
    Secp256k1,
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 2))]
    Ethereum,
}

/// A public key that can be associated with a [`Username`](crate::auth::Username).
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum Key {
    /// An Secp256r1 public key in compressed form.
    Secp256r1(ByteArray<33>),
    /// An Secp256k1 public key in compressed form.
    Secp256k1(ByteArray<33>),
    /// An Ethereum address.
    ///
    /// Ethereum uses Secp256k1 public keys, so why don't just use that? This is
    /// because Ethereum wallets typically don't expose an API that allows a
    /// webapp to know the public key. However, they do allow webapps to know the
    /// address.
    ///
    /// A webapp can technically still know the pubkey by prompting the user to
    /// sign a message, and extracting the pubkey from the signature. This would
    /// however be a bad UX, and deter the more security-minded users.
    Ethereum(Addr),
}

impl Key {
    pub fn ty(&self) -> KeyType {
        match self {
            Key::Secp256r1(_) => KeyType::Secp256r1,
            Key::Secp256k1(_) => KeyType::Secp256k1,
            Key::Ethereum(_) => KeyType::Ethereum,
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Secp256r1(key) => write!(f, "{key}"),
            Key::Secp256k1(key) => write!(f, "{key}"),
            Key::Ethereum(addr) => write!(f, "{addr}"),
        }
    }
}

/// Data that the account expects for the transaction's [`credential`](grug::Tx::credential)
/// field.
#[grug::derive(Serde)]
pub enum Signature {
    /// An Secp256r1 signature signed by a Passkey, along with necessary metadata.
    Passkey(PasskeySignature),
    /// An Secp256k1 signature.
    Secp256k1(ByteArray<64>),
    /// An EIP712 signature signed by a compatible eth wallet.
    Eip712(Eip712Signature),
}

#[grug::derive(Serde)]
pub enum Credential {
    Standard(StandardCredential),
    Session(SessionCredential),
}

#[grug::derive(Serde)]
pub struct StandardCredential {
    /// Identifies the key which the user used to sign this transaction.
    pub key_hash: Hash256,
    /// Signature of the `SignDoc` or `SessionInfo` by the user private key.
    pub signature: Signature,
}

#[grug::derive(Serde)]
pub struct SessionCredential {
    /// The `SessionInfo` that contains data to be signed with user key and otp key.
    pub session_info: SessionInfo,
    /// Signature of the `SignDoc` by the session key.
    pub session_signature: ByteArray<64>,
    /// Signatures of the `SessionInfo` by the user key.
    pub authorization: StandardCredential,
}

#[grug::derive(Serde)]
pub struct SessionInfo {
    /// Public key of the session key.
    pub session_key: ByteArray<33>,
    /// Expiry time of the session key.
    pub expire_at: Timestamp,
}

impl SignData for SessionInfo {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> Result<Vec<u8>, Self::Error> {
        // Convert to JSON value first, then to bytes, such that the struct fields
        // are ordered alphabetically.
        self.to_json_value()?.to_json_vec()
    }
}

/// Data that a transaction's sender must sign with their private key.
///
/// This includes the messages to be included in the transaction, as well as
/// sender, metadata and sender for replay protection.
#[grug::derive(Serde)]
pub struct SignDoc {
    pub sender: Addr,
    pub gas_limit: u64,
    pub messages: NonEmpty<Vec<Message>>,
    pub data: Metadata,
}

impl SignData for SignDoc {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> Result<Vec<u8>, Self::Error> {
        // Convert to JSON value first, then to bytes, such that the struct fields
        // are ordered alphabetically.
        self.to_json_value()?.to_json_vec()
    }
}

/// Data that the account expects for the transaction's [`data`](grug::Tx::data)
/// field.
#[grug::derive(Serde)]
pub struct Metadata {
    /// Identifies the user who signed this transaction.
    pub username: Username,
    /// Identifies the chain this transaction is intended for.
    pub chain_id: String,
    /// The nonce this transaction was signed with.
    pub nonce: Nonce,
    /// The expiration time of this transaction.
    pub expiry: Option<Timestamp>,
}

/// An Secp256r1 signature generated by a Passkey via Webauthn, along with
/// necessary metadata.
#[grug::derive(Serde)]
pub struct PasskeySignature {
    pub authenticator_data: Binary,
    pub client_data: Binary,
    pub sig: ByteArray<64>,
}

/// An EIP712 signature signed with a compatible eth wallet.
#[grug::derive(Serde)]
pub struct Eip712Signature {
    /// The EIP712 typed data object containing type information, domain, and
    /// the message object.
    pub typed_data: Binary,
    /// Ethereum signature.
    ///
    /// The first 64 bytes are the typical Secp256k1 signature. The last byte
    /// is the recovery id, which can take on the values: 0, 1, 27, 28.
    pub sig: ByteArray<65>,
}

/// Passkey client data.
#[grug::derive(Serde)]
pub struct ClientData {
    // Should be "webauthn.get".
    #[serde(rename = "type")]
    pub ty: String,
    // Should be the `SignDoc` in base64 `URL_SAFE_NO_PAD` encoding.
    pub challenge: String,
    pub origin: String,
    #[serde(default, rename = "crossOrigin")]
    pub cross_origin: Option<bool>,
}
