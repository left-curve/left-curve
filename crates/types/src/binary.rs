use {
    borsh::{BorshDeserialize, BorshSerialize},
    data_encoding::BASE64,
    grug_math::Inner,
    serde::{de, ser},
    std::{
        fmt,
        ops::{Deref, DerefMut},
    },
};

#[derive(
    BorshSerialize, BorshDeserialize, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Binary(Vec<u8>);

impl Binary {
    pub fn empty() -> Self {
        Self(vec![])
    }
}

impl Inner for Binary {
    type U = Vec<u8>;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl AsRef<[u8]> for Binary {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsMut<[u8]> for Binary {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
}

impl Deref for Binary {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Binary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> From<[u8; N]> for Binary {
    fn from(array: [u8; N]) -> Self {
        Self(array.to_vec())
    }
}

impl From<Vec<u8>> for Binary {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
    }
}

impl From<&[u8]> for Binary {
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}

impl From<String> for Binary {
    fn from(string: String) -> Self {
        Self(string.into_bytes())
    }
}

impl From<&str> for Binary {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl From<Binary> for Vec<u8> {
    fn from(binary: Binary) -> Self {
        binary.0
    }
}

impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64.encode(&self.0))
    }
}

impl fmt::Debug for Binary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Binary({})", BASE64.encode(&self.0))
    }
}

impl ser::Serialize for Binary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&BASE64.encode(&self.0))
    }
}

impl<'de> de::Deserialize<'de> for Binary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(BinaryVisitor)
    }
}

struct BinaryVisitor;

impl<'de> de::Visitor<'de> for BinaryVisitor {
    type Value = Binary;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A base64 encoded string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BASE64
            .decode(v.as_bytes())
            .map(Binary)
            .map_err(|err| E::custom(format!("Invalid base64: {err}")))
    }
}
