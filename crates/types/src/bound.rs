use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{
        de::{self, Error},
        Serialize,
    },
    std::{io, marker::PhantomData, ops::Deref},
};

/// A limit for a value.
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
#[derive(Serialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    pub fn inner(&self) -> &T {
        &self.value
    }

    pub fn into_inner(self) -> T {
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::JsonDeExt,
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
    fn parsing_fee_rate() {
        assert!(FeeRate::new(Udec256::new_percent(0_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(50_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(100_u128)).is_err());

        assert!("\"0\"".deserialize_json::<FeeRate>().is_ok());
        assert!("\"0.5\"".deserialize_json::<FeeRate>().is_ok());
        assert!("\"1\"".deserialize_json::<FeeRate>().is_err());
    }
}
