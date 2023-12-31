use {
    crate::{MapKey, RawKey},
    anyhow::bail,
    serde::{de, ser},
    std::{fmt, str::FromStr},
};

// comparing addresses in cosmwasm vs in CWD
// - in cosmwasm: 20 bytes, bech32 encoding, Addr is a wrapper of String
// - in CWD: 32 bytes, hex encoding (lowercase, no checksum, with 0x prefix),
//   Addr is a wrapper of [u8; 32]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Addr([u8; Self::LENGTH]);

impl Addr {
    /// We use HEX encoding for addresses (lowercase, no checksum), with the 0x prefix.
    pub const PREFIX: &'static str = "0x";

    /// The length (number of bytes) of addresses.
    ///
    /// In CWD, an address is a BLAKE3 hash of the account's instantiation data,
    /// so the length is BLAKE3's output length: 32 bytes.
    ///
    /// Do not confuse length in terms of bytes and in terms of ASCII chars.
    pub const LENGTH: usize = blake3::OUT_LEN;

    /// Generate a mock address from use in testing.
    pub const fn mock(index: u8) -> Self {
        let mut bytes = [0u8; Self::LENGTH];
        bytes[Self::LENGTH - 1] = index;
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for Addr {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let Ok(bytes) = bytes.try_into() else {
            bail!("[Addr]: incorrect length! expecting {}, found {}", Self::LENGTH, bytes.len());
        };

        Ok(Self(bytes))
    }
}

impl FromStr for Addr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(hex_str) = s.strip_prefix(Self::PREFIX) else {
            bail!("[Addr]: string does not start with expected prefix");
        };

        hex::decode(hex_str)?.as_slice().try_into()
    }
}

impl MapKey for &Addr {
    type Prefix = ();
    type Suffix = ();
    type Output = Addr;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(&self.0)]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self::Output> {
        bytes.try_into()
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, hex::encode(self.0))
    }
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Addr({}{})", Self::PREFIX, hex::encode(self.0))
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
        f.write_str("A lowercase, hex encoded, 0x prefixed string representing 32 bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Addr::from_str(v)
            .map_err(|err| E::custom(format!("[Addr]: failed to parse from string `{v}`: {err}")))
    }
}
