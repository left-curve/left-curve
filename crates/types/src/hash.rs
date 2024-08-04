use {
    crate::{forward_ref_partial_eq, StdError},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, ser},
    std::{
        fmt,
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

pub type Hash160 = Hash<20>;

pub type Hash256 = Hash<32>;

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash<const N: usize>(pub(crate) [u8; N]);

forward_ref_partial_eq!(Hash160, Hash160);

forward_ref_partial_eq!(Hash256, Hash256);

impl<const N: usize> Hash<N> {
    /// The length (number of bytes) of hashes.
    ///
    /// In Grug, we use SHA-256 hash everywhere, of which the length is 32 bytes.
    ///
    /// Do not confuse length in terms of bytes and in terms of ASCII characters.
    /// We use Hex encoding, which uses 2 ASCII characters per byte, so the
    /// ASCII length should be 64.
    pub const LENGTH: usize = N;
    /// A zeroed-out hash. Useful as mockups or placeholders.
    pub const ZERO: Self = Self([0; N]);
}

impl<const N: usize> Hash<N> {
    pub const fn from_array(slice: [u8; N]) -> Self {
        Self(slice)
    }

    pub fn into_array(self) -> [u8; N] {
        self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.into()
    }
}

impl<const N: usize> AsRef<[u8]> for Hash<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> AsMut<[u8]> for Hash<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl<const N: usize> Deref for Hash<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for Hash<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for Hash<N> {
    type Error = StdError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self, _>(
                "hash is not of the correct length",
            ));
        };

        Ok(Self(bytes))
    }
}

impl<const N: usize> TryFrom<&[u8]> for Hash<N> {
    type Error = StdError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            return Err(StdError::deserialize::<Self, _>(
                "hash is not of the correct length",
            ));
        };

        Ok(Self(bytes))
    }
}

impl<const N: usize> FromStr for Hash<N> {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(StdError::deserialize::<Self, _>(
                "hash must only contain uppercase alphanumeric characters",
            ));
        }

        hex::decode(s)?.as_slice().try_into()
    }
}

impl<const N: usize> fmt::Display for Hash<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode_upper(self.0))
    }
}

impl<const N: usize> fmt::Debug for Hash<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", hex::encode_upper(self.0))
    }
}

impl<const N: usize> ser::Serialize for Hash<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, const N: usize> de::Deserialize<'de> for Hash<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(HashVisitor::<N>)
    }
}

struct HashVisitor<const N: usize>;

impl<'de, const N: usize> de::Visitor<'de> for HashVisitor<N> {
    type Value = Hash<N>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an uppercase, hex-encoded string representing a hash")
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
        crate::{from_json_value, to_json_value, Hash256},
        hex_literal::hex,
        serde_json::json,
        std::str::FromStr,
    };

    // just a random block hash I grabbed from MintScan
    const MOCK_JSON: &str = "299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b";
    const MOCK_HASH: Hash256 = Hash256::from_array(hex!(
        "299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b"
    ));

    #[test]
    fn serializing() {
        assert_eq!(MOCK_JSON, MOCK_HASH.to_string());
        assert_eq!(json!(MOCK_JSON), to_json_value(&MOCK_HASH).unwrap());
    }

    #[test]
    fn deserializing() {
        assert_eq!(MOCK_HASH, Hash256::from_str(MOCK_JSON).unwrap());
        assert_eq!(
            MOCK_HASH,
            from_json_value::<Hash256>(json!(MOCK_JSON)).unwrap()
        );

        // uppercase hex strings are not accepted
        let illegal_json = json!(MOCK_JSON.to_uppercase());
        assert!(from_json_value::<Hash256>(illegal_json).is_err());

        // incorrect length
        // trim the last two characters, so the string only represents 31 bytes
        let illegal_json = json!(MOCK_JSON[..MOCK_JSON.len() - 2]);
        assert!(from_json_value::<Hash256>(illegal_json).is_err());
    }
}
