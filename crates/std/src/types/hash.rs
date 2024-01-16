use {
    crate::{MapKey, RawKey},
    anyhow::bail,
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(pub(crate) [u8; Self::LENGTH]);

impl Hash {
    /// We use lowercase HEX encoding for hashes, with the 0x prefix.
    pub const PREFIX: &'static str = "0x";

    /// The length (number of bytes) of hashes.
    ///
    /// In CWD, we use SHA-256 hash everywhere, of which the length is 32 bytes.
    ///
    /// Do not confuse length in terms of bytes and in terms of ASCII chars.
    pub const LENGTH: usize = 32;
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

impl TryFrom<&[u8]> for Hash {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            bail!("[Hash]: incorrect length! expecting {}, found {}", Self::LENGTH, bytes.len());
        };

        Ok(Self(bytes))
    }
}

impl FromStr for Hash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(hex_str) = s.strip_prefix(Self::PREFIX) else {
            bail!("[Hash]: string does not start with expected prefix");
        };

        hex::decode(hex_str)?.as_slice().try_into()
    }
}

impl MapKey for &Hash {
    type Prefix = ();
    type Suffix = ();
    type Output = Hash;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(&self.0)]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self::Output> {
        bytes.try_into()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, hex::encode(self.0))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({}{})", Self::PREFIX, hex::encode(self.0))
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
        f.write_str("A lowercase, hex encoded, 0x prefixed string representing 32 bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hash::from_str(v)
            .map_err(|err| E::custom(format!("[Hash]: failed to parse from string `{v}`: {err}")))
    }
}
