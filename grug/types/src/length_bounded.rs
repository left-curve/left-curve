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
    T: LengthBounds,
{
    pub fn new(value: T) -> StdResult<Self> {
        let length = value.length();

        if length < MIN {
            return Err(StdError::length_out_of_bound::<T>(length, "<", MIN));
        }

        if length > MAX {
            return Err(StdError::length_out_of_bound::<T>(length, ">", MAX));
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
    T: LengthBounds + Deserialize<'de>,
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
    T: LengthBounds + BorshDeserialize,
{
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let value = BorshDeserialize::deserialize_reader(reader)?;

        Self::new(value).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

// ---------------------------- LengthBounds impls -----------------------------

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

impl LengthBounds for &[u8] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<const LEN: usize> LengthBounds for [u8; LEN] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl LengthBounds for &str {
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

// -------------------------------- conversions --------------------------------

/// Conversion trait for types that can be converted into a `LengthBounded` type.
/// Is not possible to implement `From/Into` trait for generics because of
/// core conflicting implementations.
pub trait TryIntoLengthed<T, const MIN: usize, const MAX: usize>
where
    T: LengthBounds,
{
    fn try_into_lengthed(self) -> StdResult<LengthBounded<T, MIN, MAX>>;
}

impl<T, U, const MIN: usize, const MAX: usize> TryIntoLengthed<T, MIN, MAX> for U
where
    T: LengthBounds,
    U: Into<T>,
{
    fn try_into_lengthed(self) -> StdResult<LengthBounded<T, MIN, MAX>> {
        LengthBounded::new(self.into())
    }
}

impl<T, U, const LEN: usize> From<[U; LEN]> for FixedLength<T, LEN>
where
    T: LengthBounds,
    [U; LEN]: Into<T>,
{
    fn from(value: [U; LEN]) -> Self {
        FixedLength::new_unchecked(value.into())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::LengthBounded,
        crate::{btree_map, Binary, ResultExt},
        paste::paste,
    };

    macro_rules! valid {
        (
            $(
                {
                    value = $value:expr,
                    min   = $min:expr,
                    max   = $max:expr,
                    name  = $name:literal $(,)?
                }
            ),* $(,)?
        ) => {
            paste! {
                $(
                    #[test]
                    fn [<length_bounded_ok_ $name>]() {
                        LengthBounded::<_, $min, $max>::new($value).unwrap();
                    }
                )*
            }

        };
    }

    macro_rules! invalid {
        (
            $(
                {
                    value = $value:expr,
                    min   = $min:expr,
                    max   = $max:expr,
                    error = $error:expr,
                    name  = $name:literal $(,)?
                }
            ),* $(,)?
        ) => {
            paste! {
                $(
                    #[test]
                    fn [<length_bounded_err_ $name>]() {
                        LengthBounded::<_, $min, $max>::new($value).should_fail_with_error($error);
                    }
                )*
            }

        };
    }

    valid!(
        {
            value = "hello",
            min = 5,
            max = 6,
            name = "str_min",
        },
        {
            value = "hello",
            min = 4,
            max = 5,
            name = "str_max",
        },
        {
            value = "hello",
            min = 5,
            max = 5,
            name = "str_exact",
        },
        {
            value = Binary::from([1,2,3]),
            min = 3,
            max = 5,
            name = "binary_min",
        },
        {
            value = Binary::from([1,2,3,4,5]),
            min = 3,
            max = 5,
            name = "binary_max",
        },
        {
            value = Binary::from([1,2,3,4]),
            min = 4,
            max = 4,
            name = "binary_exact",
        },
        {
            value = btree_map!("a" => 1, "b" => 2, "c" => 3),
            min = 3,
            max = 5,
            name = "btree_map_min",
        },
        {
            value = btree_map!("a" => 1, "b" => 2, "c" => 3),
            min = 1,
            max = 3,
            name = "btree_map_max",
        },
        {
            value = btree_map!("a" => 1, "b" => 2, "c" => 3, "d" => 4),
            min = 4,
            max = 4,
            name = "btree_map_exact",
        }
    );

    invalid!(
        {
            value = "hello",
            min = 6,
            max = 8,
            error = "length of &str out of bound: 5 < 6",
            name = "str_to_short",
        },
        {
            value = "hello",
            min = 3,
            max = 4,
            error = "length of &str out of bound: 5 > 4",
            name = "str_to_long",
        },
        {
            value = Binary::from([1,2,3]),
            min = 4,
            max = 5,
            error = "length of grug_types::binary::Binary out of bound: 3 < 4",
            name = "binary_to_short",
        },
        {
            value = Binary::from([1,2,3,4,5,6]),
            min = 4,
            max = 5,
            error = "length of grug_types::binary::Binary out of bound: 6 > 5",
            name = "binary_to_long",
        },
        {
            value = btree_map!("a" => 1, "b" => 2, "c" => 3),
            min = 4,
            max = 5,
            error = "length of alloc::collections::btree::map::BTreeMap<&str, i32> out of bound: 3 < 4",
            name = "btree_map_to_short",
        },
        {
            value = btree_map!("a" => 1, "b" => 2, "c" => 3, "d" => 4, "e" => 5, "f" => 6),
            min = 4,
            max = 5,
            error = "length of alloc::collections::btree::map::BTreeMap<&str, i32> out of bound: 6 > 5",
            name = "btree_map_to_long",
        },
    );
}
