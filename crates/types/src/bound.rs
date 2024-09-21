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

/// Declare a bounded type with the given type and bounds.
#[macro_export]
macro_rules! declare_bounded {
    (name = $name:ident,type = $type:ty,min = $min:expr,max = $max:expr $(,)?) => {
        paste! {
            struct [<$name Bounds>];

            impl Bounds<$type> for [<$name Bounds>] {
                const MIN: Bound<$type> = $min;
                const MAX: Bound<$type> = $max;
            }

            type $name = Bounded<$type, [<$name Bounds>]>;
        }
    };
    (name = $name:ident,type = $type:ty,max = $max:expr $(,)?) => {
        declare_bounded! {
            name = $name,
            type = $type,
            min = Bound::Unbounded,
            max = $max,
        }
    };
    (name = $name:ident,type = $type:ty,min = $max:expr $(,)?) => {
        declare_bounded! {
            name = $name,
            type = $type,
            min = $min,
            max = Bound::Unbounded,
        }
    };
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_math::{NumberConst, Udec256, Uint256},
        paste::paste,
    };

    declare_bounded! {
        name = FeeRate,
        type = Udec256,
        min = Bound::Included(Udec256::ZERO),
        // TODO: we need an easier way of defining const Uint256
        max = Bound::Excluded(Udec256::raw(Uint256::new_from_u128(1_000_000_000_000_000_000))),
    }

    #[test]
    fn parsing_fee_rate() {
        // Ensure the `FeeRateBounds` type is correctly defined.
        assert_eq!(FeeRateBounds::MIN, Bound::Included(Udec256::ZERO));
        assert_eq!(
            FeeRateBounds::MAX,
            Bound::Excluded(Udec256::new_percent(100_u128))
        );

        // Attempt to parse various values into `FeeRate`.
        assert!(FeeRate::new(Udec256::new_percent(0_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(50_u128)).is_ok());
        assert!(FeeRate::new(Udec256::new_percent(100_u128)).is_err());
    }
}
