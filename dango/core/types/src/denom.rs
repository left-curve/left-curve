use {
    crate::{Inner, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{
        Serialize,
        de::{self, Error},
        ser,
    },
    std::{
        fmt::{self, Display, Formatter, Write},
        io,
        ops::Deref,
        str::FromStr,
    },
};

// ----------------------------------- part ------------------------------------

/// A non-empty, alphanumeric string; makes up coin denoms.
#[derive(Serialize, BorshSerialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Part(String);

impl Part {
    pub fn new_unchecked<T>(s: T) -> Self
    where
        T: Into<String>,
    {
        Self(s.into())
    }
}

impl Inner for Part {
    type U = String;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl AsRef<str> for Part {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for Part {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Part {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl TryFrom<String> for Part {
    type Error = StdError;

    fn try_from(s: String) -> StdResult<Self> {
        if s.is_empty() {
            return Err(StdError::invalid_denom(s, "empty part"));
        }

        if s.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
            return Err(StdError::invalid_denom(s, "non-alphanumeric character"));
        }

        Ok(Self(s))
    }
}

impl TryFrom<&str> for Part {
    type Error = StdError;

    fn try_from(s: &str) -> StdResult<Self> {
        Part::try_from(s.to_string())
    }
}

impl FromStr for Part {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        Part::try_from(s.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Part {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        <String as de::Deserialize>::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

impl BorshDeserialize for Part {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        <String as BorshDeserialize>::deserialize_reader(reader)?
            .try_into()
            .map_err(io::Error::other)
    }
}

// ----------------------------------- denom -----------------------------------

/// Denomination of a coin.
///
/// A valid denom that is no longer than 128 characters, consisting of one or
/// more parts, each an ASCII alphanumeric string (`a-z|A-Z|0-9`), separated by
/// the forward slash (`/`).
///
/// Examples of valid denoms:
///
/// - `uosmo`
/// - `gamm/pool/1234`
///
/// Examples of invalid denoms:
///
/// - `` (empty)
/// - `aaa...aaa` (>128 `a`'s; too long)
/// - `gamm//1234` (empty part)
/// - `gamm/&/1234` (non-alphanumeric character)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Denom(Vec<Part>);

impl Denom {
    pub fn from_parts<T, I>(parts: I) -> StdResult<Self>
    where
        T: TryInto<Part>,
        I: IntoIterator<Item = T>,
        StdError: From<T::Error>,
    {
        let denom = parts
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .map(Denom)?;

        if !(1..=128).contains(&denom.to_string().len()) {
            return Err(StdError::invalid_denom(denom, "too short or too long"));
        }

        Ok(denom)
    }

    pub fn new_unchecked<T, I>(parts: I) -> Self
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        Self(parts.into_iter().map(Part::new_unchecked).collect())
    }

    /// Return the denom's namespace.
    ///
    /// A denom's namespace is its first part, if it has more than one part.
    /// A denom consisting of only one part is considered to be under the "top-level
    /// namespace", in which case this method returns `None`.
    pub fn namespace(&self) -> Option<&Part> {
        if self.0.len() > 1 {
            Some(&self.0[0])
        } else {
            None
        }
    }

    /// Return whether the denom is prefixed with the given parts.
    pub fn starts_with(&self, parts: &[Part]) -> bool {
        self.0.starts_with(parts)
    }

    /// Prepend a slice of parts to the beginning of the denom.
    ///
    /// Fails if the new denom is too long.
    pub fn prepend(&self, parts: &[&Part]) -> StdResult<Denom> {
        let mut parts = parts.iter().copied().cloned().collect::<Vec<_>>();
        parts.extend_from_slice(&self.0);
        Denom::from_parts(parts)
    }

    /// Remove the given prefix, return the rest as a new denom.
    ///
    /// Return `None` if the `self` does not start with the given prefix.
    pub fn strip(&self, parts: &[&Part]) -> Option<Denom> {
        // We don't use `start_with` because it takes a &[Part] instead of &[&Part].
        // Cloning the parts would be expensive.
        if self.0.iter().zip(parts).all(|(a, b)| a == *b) {
            Some(Denom(self.0[parts.len()..].to_vec()))
        } else {
            None
        }
    }
}

impl Inner for Denom {
    type U = Vec<Part>;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl Display for Denom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0[0].as_str())?;
        for part in &self.0[1..] {
            f.write_char('/')?;
            f.write_str(part.as_str())?;
        }
        Ok(())
    }
}

impl FromStr for Denom {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        if !(1..=128).contains(&s.len()) {
            return Err(StdError::invalid_denom(s, "too short or too long"));
        }

        s.split('/')
            .map(Part::from_str)
            .collect::<StdResult<Vec<_>>>()
            .map(Self)
    }
}

impl TryFrom<&str> for Denom {
    type Error = StdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Denom::from_str(s)
    }
}

impl TryFrom<String> for Denom {
    type Error = StdError;

    fn try_from(s: String) -> StdResult<Self> {
        Denom::from_str(s.as_str())
    }
}

impl ser::Serialize for Denom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Denom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        <String as de::Deserialize>::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

impl BorshSerialize for Denom {
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        BorshSerialize::serialize(&self.to_string(), writer)
    }
}

impl BorshDeserialize for Denom {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        <String as BorshDeserialize>::deserialize_reader(reader)?
            .try_into()
            .map_err(io::Error::other)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{BorshDeExt, BorshSerExt, Denom, JsonDeExt, JsonSerExt, ResultExt},
        std::str::FromStr,
        test_case::test_case,
    };

    #[test_case(
        "uosmo",
        Result::Ok(Denom::new_unchecked(["uosmo"]));
        "valid denom with one part"
    )]
    #[test_case(
        "gamm/pool/1234",
        Result::Ok(Denom::new_unchecked(["gamm", "pool", "1234"]));
        "valid denom with multiple parts"
    )]
    #[test_case(
        "",
        Result::Err("too short or too long");
        "empty denom"
    )]
    #[test_case(
        "a".repeat(129),
        Result::Err("too short or too long");
        "invalid denom that is too long"
    )]
    #[test_case(
        "gamm//1234",
        Result::Err("empty part");
        "invalid denom with empty subdenom"
    )]
    #[test_case(
        "gamm/&/1234",
        Result::Err("non-alphanumeric character");
        "invalid denom with non-alphanumeric character"
    )]
    fn creating_denom_from_string<T>(input: T, expect: Result<Denom, &'static str>)
    where
        T: AsRef<str>,
    {
        Denom::from_str(input.as_ref()).should_match(expect)
    }

    #[test_case(
        Denom::new_unchecked(["uosmo"]),
        "\"uosmo\"";
        "denom with one part"
    )]
    #[test_case(
        Denom::new_unchecked(["gamm", "pool", "1234"]),
        "\"gamm/pool/1234\"";
        "denom with multiple parts"
    )]
    fn serializing_json(denom: Denom, string: &str) {
        denom
            .to_json_vec()
            .should_succeed_and_equal(string.as_bytes());
        string
            .deserialize_json::<Denom>()
            .should_succeed_and_equal(denom);
    }

    #[test_case(
        Denom::new_unchecked(["uosmo"]);
        "denom with one part"
    )]
    #[test_case(
        Denom::new_unchecked(["gamm", "pool", "1234"]);
        "denom with multiple parts"
    )]
    fn serializing_borsh(denom: Denom) {
        denom
            .to_borsh_vec()
            .unwrap()
            .deserialize_borsh::<Denom>()
            .should_succeed_and_equal(denom);
    }
}
