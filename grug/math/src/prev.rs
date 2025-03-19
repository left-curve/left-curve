use {
    crate::{
        Dec128, Dec256, Inner, Int64, Int128, Int256, Int512, MathError, MathResult, Udec128,
        Udec256, Uint64, Uint128, Uint256, Uint512,
    },
    bnum::BTryFrom,
};

/// Describes a number type can be cast into another type of a smaller word size.
///
/// For example, [`Uint256`](crate::Uint256) can be cast to [`Uint128`](crate::Uint128).
/// In this case, [`PrevNumber`] trait should be implemented for [`Uint256`](crate::Uint256)
/// with `Prev` being [`Uint128`](crate::Uint128).
pub trait PrevNumber {
    type Prev;

    fn checked_into_prev(self) -> MathResult<Self::Prev>;
}

// ------------------------------------ std ------------------------------------

macro_rules! impl_prev {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn checked_into_prev(self) -> MathResult<Self::Prev> {
                self.0.try_into().map(<$prev>::new).map_err(|_| {
                    MathError::overflow_conversion::<_, $prev>(self)
                })
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev!($this => $prev);
        )+
    };
}

impl_prev! {
    Uint128 => Uint64,
    Uint256 => Uint128,
    Int128  => Int64,
    Int256  => Int128,
}

// ----------------------------------- bnum ------------------------------------

macro_rules! impl_prev_bnum {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn checked_into_prev(self) -> MathResult<Self::Prev> {
                BTryFrom::<<$this as Inner>::U>::try_from(self.0)
                    .map(<$prev>::new)
                    .map_err(|_| MathError::overflow_conversion::<_, Uint256>(self))
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev_bnum!($this => $prev);
        )+
    };
}

impl_prev_bnum! {
    Uint512 => Uint256,
    Int512  => Int256,
}

// ----------------------------------- dec -------------------------------------

macro_rules! impl_prev_dec {
    ($this:ty => $prev:ty) => {
        impl PrevNumber for $this {
            type Prev = $prev;

            fn checked_into_prev(self) -> MathResult<Self::Prev> {
                self.0.checked_into_prev().map(<$prev>::raw)
            }
        }
    };
    ($($this:ty => $prev:ty),+ $(,)?) => {
        $(
            impl_prev_dec!($this => $prev);
        )+
    };
}

impl_prev_dec! {
    Udec256 => Udec128,
    Dec256  => Dec128,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{Int, MathError, PrevNumber, int_test, test_utils::bt},
        bnum::types::{I256, U256},
    };

    int_test!( prev
        inputs = {
            u128 = {
                passing: [
                    (u64::MAX as u128, u64::MAX),
                ],
                failing: [
                    u64::MAX as u128 + 1,
                ]
            }
            u256 = {
                passing: [
                    (U256::from(u128::MAX), u128::MAX),
                ],
                failing: [
                    U256::from(u128::MAX) + U256::ONE,
                ]
            }
            i128 = {
                passing: [
                    (i64::MAX as i128, i64::MAX),
                    (i64::MIN as i128, i64::MIN),
                ],
                failing: [
                    i64::MAX as i128 + 1,
                    i64::MIN as i128 - 1,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(i128::MAX), i128::MAX),
                    (I256::from(i128::MIN), i128::MIN),
                ],
                failing: [
                    I256::from(i128::MAX) + I256::ONE,
                    I256::from(i128::MIN) - I256::ONE,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (current, prev) in passing {
                let current = bt(_0, Int::new(current));
                assert_eq!(current.checked_into_prev().unwrap(), Int::new(prev));
            }

            for failing in failing {
                let current = bt(_0, Int::new(failing));
                assert!(matches!(current.checked_into_prev(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use {
        crate::{Dec, Int, MathError, PrevNumber, dec_test, test_utils::bt},
        bnum::types::{I256, U256},
    };

    dec_test!( prev
        inputs = {
            udec256 = {
                passing: [
                    (U256::from(u128::MAX), u128::MAX),
                ],
                failing: [
                    U256::from(u128::MAX) + U256::ONE,
                ]
            }
            dec256 = {
                passing: [
                    (I256::from(i128::MAX), i128::MAX),
                    (I256::from(i128::MIN), i128::MIN),
                ],
                failing: [
                    I256::from(i128::MAX) + I256::ONE,
                    I256::from(i128::MIN) - I256::ONE,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (current, prev) in passing {
                let current = bt(_0d, Dec::raw(bt(_0d.0, Int::new(current))));
                assert_eq!(current.checked_into_prev().unwrap(), Dec::raw(Int::new(prev)));
            }

            for failing in failing {
                let failing = bt(_0d, Dec::raw(bt(_0d.0, Int::new(failing))));
                assert!(matches!(failing.checked_into_prev(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}
