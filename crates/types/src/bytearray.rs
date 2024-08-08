use {
    crate::{Binary, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    data_encoding::BASE64,
    serde::{de, ser},
    std::{
        fmt,
        ops::{Deref, DerefMut},
    },
};

/// Fixed-length, stack-allocated, base64-encoded byte array.
///
/// Similar to [`Binary`](crate::Binary), except for this is fixed length.
///
/// Useful for defining data of known lengths, such as public keys and signatures.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteArray<const N: usize>([u8; N]);

impl<const N: usize> ByteArray<N> {
    pub fn into_array(self) -> [u8; N] {
        self.0
    }
}

impl<const N: usize> AsRef<[u8]> for ByteArray<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> AsMut<[u8]> for ByteArray<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl<const N: usize> Deref for ByteArray<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for ByteArray<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> From<[u8; N]> for ByteArray<N> {
    fn from(array: [u8; N]) -> Self {
        Self(array)
    }
}

impl<const N: usize> TryFrom<&[u8]> for ByteArray<N> {
    type Error = StdError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        slice.try_into().map(Self).map_err(StdError::TryFromSlice)
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for ByteArray<N> {
    type Error = StdError;

    fn try_from(vec: Vec<u8>) -> StdResult<Self> {
        vec.as_slice().try_into()
    }
}

impl<const N: usize> TryFrom<Binary> for ByteArray<N> {
    type Error = StdError;

    fn try_from(binary: Binary) -> StdResult<Self> {
        binary.as_ref().try_into()
    }
}

impl<const N: usize> fmt::Display for ByteArray<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", BASE64.encode(&self.0))
    }
}

impl<const N: usize> fmt::Debug for ByteArray<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ByteArray::<{}>({})", N, BASE64.encode(&self.0))
    }
}

impl<const N: usize> ser::Serialize for ByteArray<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_some(&BASE64.encode(&self.0))
    }
}

impl<'de, const N: usize> de::Deserialize<'de> for ByteArray<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(ByteArrayVisitor::<N>)
    }
}

struct ByteArrayVisitor<const N: usize>;

impl<'de, const N: usize> de::Visitor<'de> for ByteArrayVisitor<N> {
    type Value = ByteArray<N>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A base64-encoded string representing {N} bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BASE64
            .decode(v.as_bytes())
            .map_err(E::custom)?
            .try_into()
            .map_err(E::custom)
    }
}
