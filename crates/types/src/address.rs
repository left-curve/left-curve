use {
    crate::{forward_ref_partial_eq, hash160, hash256, Hash, Hash160, Hash256, StdError},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        fmt,
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

/// In Grug, addresses are of 20-byte length, in lowercase Hex encoding with the
/// `0x` prefix. Checksums are included as described by
/// [EIP-55](https://github.com/ethereum/ercs/blob/master/ERCS/erc-55.md).
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
/// In Grug, addresses are validated during deserialization. If deserialization
/// doesn't throw an error, you can be sure the address is valid. Therefore it
/// is safe to use `Addr`s in JSON messages.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Addr(pub(crate) Hash160);

forward_ref_partial_eq!(Addr, Addr);

impl Addr {
    /// Addresses are encoded as lowercase hex strings, with the 0x prefix.
    pub const PREFIX: &'static str = "0x";

    /// Create a new address from a 32-byte byte slice.
    pub const fn from_array(slice: [u8; Hash160::LENGTH]) -> Self {
        Self(Hash160::from_array(slice))
    }

    /// Compute a contract address as:
    ///
    /// ```plain
    /// address := ripemd160(sha256(deployer_addr | code_hash | salt))
    /// ```
    ///
    /// where `|` means byte concatenation.
    ///
    /// The double hash the same as used by Bitcoin, for [preventing length
    /// extension attacks](https://bitcoin.stackexchange.com/questions/8443/where-is-double-hashing-performed-in-bitcoin).
    pub fn compute(deployer: &Addr, code_hash: &Hash256, salt: &[u8]) -> Self {
        let mut preimage = Vec::with_capacity(Hash160::LENGTH + Hash256::LENGTH + salt.len());
        preimage.extend_from_slice(deployer.as_ref());
        preimage.extend_from_slice(code_hash.as_ref());
        preimage.extend_from_slice(salt);
        Self(hash160(hash256(preimage)))
    }

    /// Generate a mock address from use in testing.
    pub const fn mock(index: u8) -> Self {
        let mut bytes = [0u8; Hash160::LENGTH];
        bytes[Hash160::LENGTH - 1] = index;
        Self(Hash160::from_array(bytes))
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

impl TryFrom<Vec<u8>> for Addr {
    type Error = StdError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self, _>(
                "address is not of the correct length",
            ));
        };

        Ok(Self(Hash(bytes)))
    }
}

impl TryFrom<&[u8]> for Addr {
    type Error = StdError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self, _>(
                "address is not of the correct length",
            ));
        };

        Ok(Self(Hash(bytes)))
    }
}

impl FromStr for Addr {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(hex_str) = s.strip_prefix(Self::PREFIX) else {
            return Err(StdError::deserialize::<Self, _>(
                "address must use the 0x prefix",
            ));
        };

        Hash::from_str(hex_str).map(Self)
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
        crate::{from_json_value, to_json_value, Addr},
        hex_literal::hex,
        serde_json::json,
        std::str::FromStr,
    };

    // `vitalik.eth`
    const MOCK_STR: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    const MOCK_ADDR: Addr = Addr::from_array(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045"));

    #[test]
    fn serializing() {
        assert_eq!(MOCK_STR, MOCK_ADDR.to_string());
        assert_eq!(json!(MOCK_STR), to_json_value(&MOCK_ADDR).unwrap());
    }

    #[test]
    fn deserializing() {
        assert_eq!(MOCK_ADDR, Addr::from_str(MOCK_STR).unwrap());
        assert_eq!(MOCK_ADDR, from_json_value::<Addr>(json!(MOCK_STR)).unwrap());
    }
}
