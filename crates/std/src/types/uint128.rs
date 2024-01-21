use {
    crate::{StdError, StdResult},
    serde::{de, ser},
    std::fmt,
};

/// A wrapper of uint128, serialized as a string.
///
/// JSON supports integer numbers in the range of [-(2^53)+1, (2^53)-1].
/// Numbers beyond this range (uint64, uint128...) need to serialize as strings.
/// https://stackoverflow.com/questions/13502398/json-integers-limit-on-size#comment80159722_13502497
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint128(u128);

impl Uint128 {
    pub const MAX: Self = Self::new(u128::MAX);
    pub const MIN: Self = Self::new(u128::MAX);

    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub fn u128(self) -> u128 {
        self.0
    }

    #[inline]
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    pub const fn to_le_bytes(self) -> [u8; 16] {
        self.0.to_le_bytes()
    }

    pub fn checked_add(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_add(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_add(self, other))
    }

    pub fn checked_sub(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_sub(self, other))
    }

    pub fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::overflow_mul(self, other))
    }

    pub fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self::new)
            .ok_or_else(|| StdError::division_by_zero(self))
    }

    pub fn checked_multiply_ratio(self, _other: Self) -> StdResult<Self> {
        // need Uint256 implemented
        todo!()
    }
}

// TODO: implement Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign

impl From<Uint128> for String {
    fn from(value: Uint128) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Uint128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl ser::Serialize for Uint128 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Uint128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Uint128Visitor)
    }
}

struct Uint128Visitor;

impl<'de> de::Visitor<'de> for Uint128Visitor {
    type Value = Uint128;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A string-encoded 128-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<u128>()
            .map(Uint128::new)
            .map_err(|err| E::custom(format!("[Uint128]: failed to parse from string `{v}`: {err}")))
    }
}
