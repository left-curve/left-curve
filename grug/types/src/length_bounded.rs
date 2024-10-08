use {
    crate::{Binary, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::Inner,
    serde::{Deserialize, Deserializer, Serialize, Serializer},
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::Deref,
    },
};

pub type MaxLength<T, const MAX: usize> = LengthBounded<T, 0, MAX>;
pub type MinLength<T, const MIN: usize> = LengthBounded<T, MIN, { usize::MAX }>;
pub type NonEmpty<T> = MinLength<T, 1>;
pub type FixedLength<T, const LEN: usize> = LengthBounded<T, LEN, LEN>;
pub type ByteArray<const LEN: usize> = FixedLength<Binary, LEN>;

/// A wrapper that enforces the value to be within the specified length.
/// The value must implement the `LengthBounds` trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LengthBounded<T, const MIN: usize, const MAX: usize>(T)
where
    T: LengthBounds;

pub trait LengthBounds {
    fn length(&self) -> usize;
}

impl<T, const MIN: usize, const MAX: usize> LengthBounded<T, MIN, MAX>
where
    T: LengthBounds + ToString,
{
    pub fn new(value: T) -> StdResult<Self> {
        let length = value.length();
        // if length < MIN || length > MAX {
        //     return Err(StdError::out_of_range(value, ">", B.to_string()))
        // }

        if length < MIN {
            return Err(StdError::lenght_out_of_bound(value, length, "<", MIN));
        }

        if length > MAX {
            return Err(StdError::lenght_out_of_bound(value, length, ">", MAX));
        }

        Ok(Self(value))
    }

    pub fn new_unchecked(value: T) -> Self {
        Self(value)
    }
}

impl<T, const MIN: usize, const MAX: usize> Inner for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds,
{
    type U = T;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> AsRef<T> for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> Deref for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> Serialize for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T, const MIN: usize, const MAX: usize> Deserialize<'de> for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds + ToString + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;

        LengthBounded::new(value).map_err(serde::de::Error::custom)
    }
}

impl<T, const MIN: usize, const MAX: usize> BorshSerialize for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds + BorshSerialize,
{
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.serialize(writer)
    }
}

impl<T, const MIN: usize, const MAX: usize> BorshDeserialize for LengthBounded<T, MIN, MAX>
where
    T: LengthBounds + ToString + BorshDeserialize,
{
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let value = BorshDeserialize::deserialize_reader(reader)?;

        Self::new(value).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

// ------------------------------- LengthBounds impls -------------------------------

impl LengthBounds for Binary {
    fn length(&self) -> usize {
        self.len()
    }
}

impl LengthBounds for String {
    fn length(&self) -> usize {
        self.len()
    }
}

impl LengthBounds for Vec<u8> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl LengthBounds for [u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<const LEN: usize> LengthBounds for [u8; LEN] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl LengthBounds for str {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V> LengthBounds for BTreeMap<K, V> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K> LengthBounds for BTreeSet<K> {
    fn length(&self) -> usize {
        self.len()
    }
}

// ----------------------------------- conversion -----------------------------------

/// Conversion trait for types that can be converted into a `LengthBounded` type.
/// Is not possible to implement `From/Into` trait for generics because of
/// core conflicting implementations.
pub trait TryIntoLenghted<T, const MIN: usize, const MAX: usize>
where
    T: LengthBounds,
{
    fn try_into_lenghted(self) -> StdResult<LengthBounded<T, MIN, MAX>>;
}

impl<T, U, const MIN: usize, const MAX: usize> TryIntoLenghted<T, MIN, MAX> for U
where
    T: LengthBounds + ToString,
    U: Into<T>,
{
    fn try_into_lenghted(self) -> StdResult<LengthBounded<T, MIN, MAX>> {
        LengthBounded::new(self.into())
    }
}

impl<T, U, const LEN: usize> From<[U; LEN]> for FixedLength<T, LEN>
where
    T: LengthBounds + ToString,
    [U; LEN]: Into<T>,
{
    fn from(value: [U; LEN]) -> Self {
        FixedLength::new_unchecked(value.into())
    }
}
