use {
    core::str,
    grug::{Inner, PrimaryKey, RawKey, StdError, StdResult},
    serde::{Serialize, de},
    std::{fmt, str::FromStr},
};

/// A name that uniquely identifies a user.
///
/// A valid username must contain only ASCII letters (`A-Z|a-z`), numbers (`0-9`),
/// or the underscore (`_`) and be between 1-15 characters.
#[grug::derive(Borsh)]
#[derive(Serialize, PartialOrd, Ord)]
pub struct Username(String);

impl Username {
    /// The maximum length allowed for usernames.
    pub const MAX_LEN: usize = 15;

    /// Return the username's length as a single byte.
    ///
    /// Usernames cannot be longer than 15 characters, so a single byte suffices.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u8 {
        debug_assert!(
            self.0.len() < 255,
            "username `{self}` is somehow longer than 255 characters"
        );

        self.0.len() as _
    }
}

impl Inner for Username {
    type U = String;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for Username {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl PrimaryKey for Username {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Borrowed(self.0.as_bytes())]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        // `StdError` doesn't compose `FromUtf8Error`, so we need to cast the
        // error to an `StdError::Deserialize`.
        // TODO: create `StdError::FromUtf8` variant?
        str::from_utf8(bytes)
            .map_err(|err| StdError::deserialize::<&str, _>("utf8", err))
            .and_then(Self::from_str)
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Username {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        if s.is_empty() {
            return Err(StdError::deserialize::<Self, _>(
                "str",
                "username can't be empty",
            ));
        }

        if s.len() > Self::MAX_LEN {
            return Err(StdError::deserialize::<Self, _>(
                "str",
                format!("username can't be longer than {} characters", Self::MAX_LEN),
            ));
        }

        if !s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            return Err(StdError::deserialize::<Self, _>(
                "str",
                "username can only contain alphanumeric characters (A-Z|a-z|0-9) or underscore",
            ));
        }

        Ok(Self(s.to_string()))
    }
}

impl<'de> de::Deserialize<'de> for Username {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

struct Visitor;

impl de::Visitor<'_> for Visitor {
    type Value = Username;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A string containing between 1 to 15 alphanumeric characters (A-Z|a-z|0-9) or underscore")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Username::from_str(v).map_err(E::custom)
    }
}
