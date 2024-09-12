use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{
        de::{self, Error},
        Serialize,
    },
    std::{
        fmt::{self, Display, Formatter},
        io,
        str::FromStr,
    },
};

/// Denomination of a coin.
///
/// A valid denom must satisfy the following criteria:
///
/// - be 3-128 character long (inclusive);
/// - contains only ASCII alphanumeric characters (`a-z|A-Z|0-9`) or the forward
///   slash (`/`);
/// - no two consecutive forward slashes;
/// - the first character must be a letter (`a-z|A-Z`).
///
/// Note that this is more strict than [Cosmos SDK's criteria](https://github.com/cosmos/cosmos-sdk/blob/v0.50.9/types/coin.go#L838),
/// so some valid Cosmos SDK denoms may not be valid Grug denoms.
#[derive(Serialize, BorshSerialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Denom(String);

impl Denom {
    /// Create a new denom from a string.
    /// Error if the string isn't a valid denom.
    pub fn new<T>(inner: T) -> StdResult<Self>
    where
        T: Into<String>,
    {
        let inner = inner.into();

        if !(3..=128_usize).contains(&inner.len()) {
            return Err(StdError::invalid_denom(inner, "too short or too long"));
        }

        if !inner.chars().next().unwrap().is_ascii_alphabetic() {
            return Err(StdError::invalid_denom(inner, "non-alphabetic lead"));
        }

        for subdenom in inner.split('/') {
            if subdenom.is_empty() {
                return Err(StdError::invalid_denom(inner, "empty subdenom"));
            }

            if subdenom.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
                return Err(StdError::invalid_denom(inner, "non-alphanumeric character"));
            }
        }

        Ok(Self(inner))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl Display for Denom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for Denom {
    type Error = StdError;

    fn try_from(string: String) -> StdResult<Self> {
        Denom::new(string)
    }
}

impl TryFrom<&str> for Denom {
    type Error = StdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Denom::new(s)
    }
}

impl FromStr for Denom {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        Denom::new(s)
    }
}

impl<'de> de::Deserialize<'de> for Denom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner = <String as de::Deserialize>::deserialize(deserializer)?;

        Denom::new(inner).map_err(D::Error::custom)
    }
}

impl BorshDeserialize for Denom {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let inner = <String as BorshDeserialize>::deserialize_reader(reader)?;

        Denom::new(inner).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}
