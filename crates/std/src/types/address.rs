use {
    crate::{Binary, Hash, MapKey, RawKey, StdError, StdResult},
    serde::{de, ser},
    sha2::{Digest, Sha256},
    std::{
        fmt,
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

/// In CWD, addresses are of 32-byte length, in lowercase Hex encoding with the
/// `0x` prefix. There is no checksum bytes. This is the same address format
/// used by Aptos and Sui.
///
/// In comparison, in the "vanilla" CosmWasm, addresses are either 20- or 32-byte,
/// in Bech32 encoding. The last 6 ASCII characters are the checksum.
///
/// In CosmWasm, when addresses are deserialized from JSON, no validation is
/// performed. An attacker can put a string that is not a valid address in a
/// message, and this would be deserialized into an `cosmwasm_std::Addr` without
/// error. Therefore, in CosmWasm, it is recommended to deserialize addresses
/// into `String`s first, then call `deps.api.addr_validate` to validate them.
/// This can be sometimes very cumbersome. It may be necessary to define two
/// versions of the same type, one "unchecked" version with `String`s, one
/// "checked" version with `Addr`s.
///
/// In CWD, addresses are validated during deserialization. If deserialization
/// doesn't throw an error, you can be sure the address is valid. Therefore it
/// is safe to use `Addr`s in JSON messages.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Addr(pub(crate) Hash);

impl Addr {
    /// Addresses are encoded as lowercase hex strings, with the 0x prefix.
    pub const PREFIX: &'static str = "0x";

    /// Create a new address from a hash.
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }

    /// Compute a contract address as:
    ///
    /// sha256(deployer_addr | code_hash | salt)
    ///
    /// where | means byte concatenation.
    pub fn compute(deployer: &Addr, code_hash: &Hash, salt: &Binary) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(deployer);
        hasher.update(code_hash);
        hasher.update(salt);
        Self(Hash(hasher.finalize().into()))
    }

    /// Generate a mock address from use in testing.
    pub const fn mock(index: u8) -> Self {
        let mut bytes = [0u8; Hash::LENGTH];
        bytes[Hash::LENGTH - 1] = index;
        Self(Hash(bytes))
    }
}

impl AsRef<[u8]> for Addr {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Addr {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Addr {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Addr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<&[u8]> for Addr {
    type Error = StdError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Hash::try_from(bytes).map(Self)
    }
}

impl FromStr for Addr {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(hex_str) = s.strip_prefix(Self::PREFIX) else {
            return Err(StdError::deserialize::<Self>("address must use the 0x prefix"));
        };

        Hash::from_str(hex_str).map(Self)
    }
}

impl MapKey for &Addr {
    type Prefix = ();
    type Suffix = ();
    type Output = Addr;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self.0.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> StdResult<Self::Output> {
        bytes.try_into()
    }
}

impl From<Addr> for String {
    fn from(addr: Addr) -> Self {
        addr.to_string()
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0)
    }
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Addr({}{})", Self::PREFIX, self.0)
    }
}

impl ser::Serialize for Addr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Addr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(AddrVisitor)
    }
}

struct AddrVisitor;

impl<'de> de::Visitor<'de> for AddrVisitor {
    type Value = Addr;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a lowercase, hex-encoded, 0x-prefixed string representing 32 bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Addr::from_str(v).map_err(E::custom)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json, to_json},
        hex_literal::hex,
    };

    // the same as the mock hash from the Hash unit tests, except with 0x prefix
    const MOCK_STR:  &str = "0x299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b";
    const MOCK_JSON: &str = "\"0x299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b\"";
    const MOCK_ADDR: Addr = Addr(Hash(hex!("299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b")));

    #[test]
    fn serializing() {
        assert_eq!(MOCK_STR, MOCK_ADDR.to_string());
        assert_eq!(MOCK_JSON.as_bytes(), to_json(&MOCK_ADDR).unwrap().as_ref());
    }

    #[test]
    fn deserializing() {
        assert_eq!(MOCK_ADDR, Addr::from_str(MOCK_STR).unwrap());
        assert_eq!(MOCK_ADDR, from_json(MOCK_JSON).unwrap());
    }
}
