use {
    crate::{forward_ref_partial_eq, StdError},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    sha2::{Digest, Sha256},
    std::{
        fmt,
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

pub fn hash(data: impl AsRef<[u8]>) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data.as_ref());
    Hash(hasher.finalize().into())
}

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(pub(crate) [u8; Self::LENGTH]);

forward_ref_partial_eq!(Hash, Hash);

impl Hash {
    /// The length (number of bytes) of hashes.
    ///
    /// In Grug, we use SHA-256 hash everywhere, of which the length is 32 bytes.
    ///
    /// Do not confuse length in terms of bytes and in terms of ASCII characters.
    /// We use Hex encoding, which uses 2 ASCII characters per byte, so the
    /// ASCII length should be 64.
    pub const LENGTH: usize = 32;
    /// A zeroed-out hash. Useful as mockups or placeholders.
    pub const ZERO: Self = Self([0; Self::LENGTH]);
}

impl Hash {
    pub const fn from_slice(slice: [u8; Self::LENGTH]) -> Self {
        Self(slice)
    }

    pub fn into_slice(self) -> [u8; Self::LENGTH] {
        self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.into()
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Hash {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Hash {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<Vec<u8>> for Hash {
    type Error = StdError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self>(
                "hash is not of the correct length",
            ));
        };

        Ok(Self(bytes))
    }
}

impl TryFrom<&[u8]> for Hash {
    type Error = StdError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self>(
                "hash is not of the correct length",
            ));
        };

        Ok(Self(bytes))
    }
}

impl FromStr for Hash {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(StdError::deserialize::<Self>(
                "hash must only contain lowercase alphanumeric characters",
            ));
        }

        hex::decode(s)?.as_slice().try_into()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", hex::encode(self.0))
    }
}

impl ser::Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(HashVisitor)
    }
}

struct HashVisitor;

impl<'de> de::Visitor<'de> for HashVisitor {
    type Value = Hash;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a lowercase, hex-encoded string representing 32 bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hash::from_str(v).map_err(E::custom)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json_value, to_json_value},
        hex_literal::hex,
        serde_json::json,
    };

    // just a random block hash I grabbed from MintScan
    const MOCK_JSON: &str = "299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b";
    const MOCK_HASH: Hash = Hash(hex!(
        "299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b"
    ));

    #[test]
    fn serializing() {
        assert_eq!(MOCK_JSON, MOCK_HASH.to_string());
        assert_eq!(json!(MOCK_JSON), to_json_value(&MOCK_HASH).unwrap());
    }

    #[test]
    fn deserializing() {
        assert_eq!(MOCK_HASH, Hash::from_str(MOCK_JSON).unwrap());
        assert_eq!(
            MOCK_HASH,
            from_json_value::<Hash>(json!(MOCK_JSON)).unwrap()
        );

        // uppercase hex strings are not accepted
        let illegal_json = json!(MOCK_JSON.to_uppercase());
        assert!(from_json_value::<Hash>(illegal_json).is_err());

        // incorrect length
        // trim the last two characters, so the string only represents 31 bytes
        let illegal_json = json!(MOCK_JSON[..MOCK_JSON.len() - 2]);
        assert!(from_json_value::<Hash>(illegal_json).is_err());
    }
}
