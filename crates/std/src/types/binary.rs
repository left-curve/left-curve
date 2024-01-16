use {
    data_encoding::BASE64,
    serde::{de, ser},
    std::{
        fmt,
        ops::{Deref, DerefMut},
    },
};

#[derive(Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Binary(Vec<u8>);

impl Binary {
    pub fn empty() -> Self {
        Self(vec![])
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
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl DerefMut for Binary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl From<Vec<u8>> for Binary {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
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
