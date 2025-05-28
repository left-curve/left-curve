use {
    crate::{Lengthy, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::Inner,
    serde::{Deserialize, Deserializer, Serialize, Serializer},
    std::{io, ops::Deref},
};

/// A wrapper that enforces the value to be no longer than a maximum length.
///
/// The maximum length is _inclusive_.
pub type MaxLength<T, const MAX: usize> = LengthBounded<T, 0, MAX>;

/// A Wrapper that enforces the value to be no shorter than a minimum length.
///
/// The minimum length is _inclusive_.
pub type MinLength<T, const MIN: usize> = LengthBounded<T, MIN, { usize::MAX }>;

/// A wrapper that enforces the value to not be empty.
pub type NonEmpty<T> = MinLength<T, 1>;

/// A wrapper that enforces the value to be of an exact length.
pub type FixedLength<T, const LEN: usize> = LengthBounded<T, LEN, LEN>;

/// A wrapper that enforces the value to be within a bound of length.
///
/// The minimum and maximum lengths are both _inclusive_.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LengthBounded<T, const MIN: usize, const MAX: usize>(T)
where
    T: Lengthy;

impl<T, const MIN: usize, const MAX: usize> LengthBounded<T, MIN, MAX>
where
    T: Lengthy,
{
    pub fn new(value: T) -> StdResult<Self> {
        let length = value.length();

        if length < MIN {
            return Err(StdError::length_out_of_range::<T>(length, "<", MIN));
        }

        if length > MAX {
            return Err(StdError::length_out_of_range::<T>(length, ">", MAX));
        }

        Ok(Self(value))
    }

    pub fn new_unchecked(value: T) -> Self {
        Self(value)
    }
}

impl<T, const MIN: usize, const MAX: usize> Inner for LengthBounded<T, MIN, MAX>
where
    T: Lengthy,
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
    T: Lengthy,
{
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> Deref for LengthBounded<T, MIN, MAX>
where
    T: Lengthy,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> Serialize for LengthBounded<T, MIN, MAX>
where
    T: Lengthy + Serialize,
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
    T: Lengthy + Deserialize<'de>,
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
    T: Lengthy + BorshSerialize,
{
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.0.serialize(writer)
    }
}

impl<T, const MIN: usize, const MAX: usize> BorshDeserialize for LengthBounded<T, MIN, MAX>
where
    T: Lengthy + BorshDeserialize,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let value = BorshDeserialize::deserialize_reader(reader)?;

        Self::new(value).map_err(io::Error::other)
    }
}

impl<T, U, const LEN: usize> From<[U; LEN]> for FixedLength<T, LEN>
where
    T: Lengthy + From<[U; LEN]>,
{
    fn from(value: [U; LEN]) -> Self {
        FixedLength::new_unchecked(value.into())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Binary, LengthBounded, ResultExt, btree_map},
        paste::paste,
    };

    macro_rules! valid_case {
        (
            value = $value:expr,
            min   = $min:expr,
            max   = $max:expr,
            name  = $name:literal $(,)?
        ) => {
            paste! {
                #[test]
                fn [<length_bounded_ok_ $name>]() {
                    LengthBounded::<_, $min, $max>::new($value).unwrap();
                }
            }
        };
    }

    macro_rules! invalid_case {
        (
            value = $value:expr,
            min   = $min:expr,
            max   = $max:expr,
            error = $error:expr,
            name  = $name:literal $(,)?
        ) => {
            paste! {
                #[test]
                fn [<length_bounded_err_ $name>]() {
                    LengthBounded::<_, $min, $max>::new($value).should_fail_with_error($error);
                }
            }
        };
    }

    valid_case! {
        value = "hello".to_string(),
        min   = 5,
        max   = 6,
        name  = "string_min",
    }

    valid_case! {
        value = "hello".to_string(),
        min   = 4,
        max   = 5,
        name  = "string_max",
    }

    valid_case! {
        value = "hello".to_string(),
        min   = 5,
        max   = 5,
        name  = "string_exact",
    }

    valid_case! {
        value = Binary::from([1, 2, 3]),
        min   = 3,
        max   = 5,
        name  = "binary_min",
    }

    valid_case! {
        value = Binary::from([1, 2, 3, 4, 5]),
        min   = 3,
        max   = 5,
        name  = "binary_max",
    }

    valid_case! {
        value = Binary::from([1, 2, 3, 4]),
        min   = 4,
        max   = 4,
        name  = "binary_exact",
    }

    valid_case! {
        value = btree_map! { "a" => 1, "b" => 2, "c" => 3 },
        min   = 3,
        max   = 5,
        name  = "btree_map_min",
    }

    valid_case! {
        value = btree_map! { "a" => 1, "b" => 2, "c" => 3 },
        min   = 1,
        max   = 3,
        name  = "btree_map_max",
    }

    valid_case! {
        value = btree_map! { "a" => 1, "b" => 2, "c" => 3, "d" => 4 },
        min   = 4,
        max   = 4,
        name  = "btree_map_exact",
    }

    invalid_case! {
        value = "hello".to_string(),
        min   = 6,
        max   = 8,
        error = "length of alloc::string::String out of range: 5 < 6",
        name  = "str_to_short",
    }

    invalid_case! {
        value = "hello".to_string(),
        min   = 3,
        max   = 4,
        error = "length of alloc::string::String out of range: 5 > 4",
        name  = "str_to_long",
    }

    invalid_case! {
        value = Binary::from([1, 2, 3]),
        min   = 4,
        max   = 5,
        error = "length of grug_types::encoded_bytes::EncodedBytes<alloc::vec::Vec<u8>, grug_types::encoders::Base64Encoder> out of range: 3 < 4",
        name  = "binary_to_short",
    }

    invalid_case! {
        value = Binary::from([1, 2, 3, 4, 5, 6]),
        min   = 4,
        max   = 5,
        error = "length of grug_types::encoded_bytes::EncodedBytes<alloc::vec::Vec<u8>, grug_types::encoders::Base64Encoder> out of range: 6 > 5",
        name  = "binary_to_long",
    }

    invalid_case! {
        value = btree_map! { "a" => 1, "b" => 2, "c" => 3 },
        min   = 4,
        max   = 5,
        error = "length of alloc::collections::btree::map::BTreeMap<&str, i32> out of range: 3 < 4",
        name  = "btree_map_to_short",
    }

    invalid_case! {
        value = btree_map! { "a" => 1, "b" => 2, "c" => 3, "d" => 4, "e" => 5, "f" => 6 },
        min   = 4,
        max   = 5,
        error = "length of alloc::collections::btree::map::BTreeMap<&str, i32> out of range: 6 > 5",
        name  = "btree_map_to_long",
    }
}
