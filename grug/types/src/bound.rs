use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{Inner, NumberConst, Udec128},
    serde::{
        Deserialize, Serialize,
        de::{self, Error},
        ser,
    },
    std::{io, marker::PhantomData, ops::Deref},
};

/// A limit for a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound<T> {
    Inclusive(T),
    Exclusive(T),
}

/// Describess a set of minimum and maximum bounds for a value.
pub trait Bounds<T> {
    const MIN: Option<Bound<T>>;
    const MAX: Option<Bound<T>>;
}

/// A wrapper that enforces the value to be within the specified bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    value: T,
    bounds: PhantomData<B>,
}

impl<T, B> Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    pub fn new(value: T) -> StdResult<Self> {
        match B::MIN {
            Some(Bound::Inclusive(bound)) if value < bound => {
                return Err(StdError::out_of_range(value, "<", bound));
            },
            Some(Bound::Exclusive(bound)) if value <= bound => {
                return Err(StdError::out_of_range(value, "<=", bound));
            },
            _ => (),
        }

        match B::MAX {
            Some(Bound::Inclusive(bound)) if value > bound => {
                return Err(StdError::out_of_range(value, ">", bound));
            },
            Some(Bound::Exclusive(bound)) if value >= bound => {
                return Err(StdError::out_of_range(value, ">=", bound));
            },
            _ => (),
        }

        Ok(Self {
            value,
            bounds: PhantomData,
        })
    }

    pub fn new_unchecked(value: T) -> Self {
        Self {
            value,
            bounds: PhantomData,
        }
    }
}

impl<T, B> Inner for Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    type U = T;

    fn inner(&self) -> &Self::U {
        &self.value
    }

    fn into_inner(self) -> Self::U {
        self.value
    }
}

impl<T, B> AsRef<T> for Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T, B> Deref for Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, B> ser::Serialize for Bounded<T, B>
where
    T: PartialOrd + ToString + ser::Serialize,
    B: Bounds<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de, T, B> de::Deserialize<'de> for Bounded<T, B>
where
    T: PartialOrd + ToString + de::Deserialize<'de>,
    B: Bounds<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;

        Self::new(value).map_err(D::Error::custom)
    }
}

impl<T, B> BorshSerialize for Bounded<T, B>
where
    T: PartialOrd + ToString + BorshSerialize,
    B: Bounds<T>,
{
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        self.value.serialize(writer)
    }
}

impl<T, B> BorshDeserialize for Bounded<T, B>
where
    T: PartialOrd + ToString + BorshDeserialize,
    B: Bounds<T>,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let value = BorshDeserialize::deserialize_reader(reader)?;

        Self::new(value).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}

// ------------------------------ Standard bounds ------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroInclusiveOneInclusive;

impl Bounds<Udec128> for ZeroInclusiveOneInclusive {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ZERO));
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroInclusiveOneExclusive;

impl Bounds<Udec128> for ZeroInclusiveOneExclusive {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ZERO));
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroExclusiveOneInclusive;

impl Bounds<Udec128> for ZeroExclusiveOneInclusive {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroExclusiveOneExclusive;

impl Bounds<Udec128> for ZeroExclusiveOneExclusive {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{JsonDeExt, JsonSerExt, ResultExt},
        grug_math::{NumberConst, Udec256, Uint256},
    };

    #[derive(Debug)]
    struct FeeRateBounds;

    impl Bounds<Udec256> for FeeRateBounds {
        // Maximum fee rate is 100% (exclusive).
        // If only there's an easier way to define a constant Udec256...
        const MAX: Option<Bound<Udec256>> = Some(Bound::Exclusive(Udec256::raw(
            Uint256::new_from_u128(1_000_000_000_000_000_000),
        )));
        // Minimum fee rate is 0% (inclusive).
        const MIN: Option<Bound<Udec256>> = Some(Bound::Inclusive(Udec256::ZERO));
    }

    type FeeRate = Bounded<Udec256, FeeRateBounds>;

    #[test]
    fn serializing_fee_rate() {
        FeeRate::new(Udec256::new_percent(0))
            .unwrap()
            .to_json_string()
            .should_succeed_and_equal("\"0\"");

        FeeRate::new(Udec256::new_percent(50))
            .unwrap()
            .to_json_string()
            .should_succeed_and_equal("\"0.5\"");
    }

    #[test]
    fn deserializing_fee_rate() {
        "\"0\""
            .deserialize_json::<FeeRate>()
            .map(Inner::into_inner)
            .should_succeed_and_equal(Udec256::new_percent(0));

        "\"0.5\""
            .deserialize_json::<FeeRate>()
            .map(Inner::into_inner)
            .should_succeed_and_equal(Udec256::new_percent(50));

        "\"1\""
            .deserialize_json::<FeeRate>()
            .should_fail_with_error(StdError::out_of_range("1", ">=", "1"));
    }
}
