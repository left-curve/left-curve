use {
    crate::{StdError, StdResult},
    std::{marker::PhantomData, ops::Bound},
};

pub trait Bounds<T> {
    const MIN: Bound<T>;
    const MAX: Bound<T>;
}

#[derive(Debug)]
pub struct Bounded<T, B>(T, PhantomData<B>);

impl<T, B> Bounded<T, B>
where
    T: PartialOrd + ToString,
    B: Bounds<T>,
{
    pub fn new(value: T) -> StdResult<Self> {
        match B::MIN {
            Bound::Included(bound) if value < bound => {
                return Err(StdError::out_of_range(value, "<", bound));
            },
            Bound::Excluded(bound) if value <= bound => {
                return Err(StdError::out_of_range(value, "<=", bound));
            },
            _ => (),
        }

        match B::MAX {
            Bound::Included(bound) if value > bound => {
                return Err(StdError::out_of_range(value, ">", bound));
            },
            Bound::Excluded(bound) if value >= bound => {
                return Err(StdError::out_of_range(value, ">=", bound));
            },
            _ => (),
        }

        Ok(Self(value, PhantomData))
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_math::{NumberConst, Udec256, Uint256},
    };

    #[derive(Debug)]
    struct FeeRateBounds;

    impl Bounds<Udec256> for FeeRateBounds {
        // Maximum fee rate is 100% (exclusive).
        // If only there's an easier way to define a constant Udec256...
        const MAX: Bound<Udec256> = Bound::Excluded(Udec256::raw(Uint256::new_from_u128(
            1_000_000_000_000_000_000,
        )));
        // Minimum fee rate is 0% (inclusive).
        const MIN: Bound<Udec256> = Bound::Included(Udec256::ZERO);
    }

    type FeeRate = Bounded<Udec256, FeeRateBounds>;

    #[test]
    fn parsing_fee_rate() {
        assert!(FeeRate::new(Udec256::new_percent(0_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(50_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(100_u128)).is_err());
    }
}
