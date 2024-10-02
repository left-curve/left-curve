use {
    crate::{
        Dec, FixedPoint, Int, Integer, IsZero, MathError, MathResult, NextNumber, NumberConst,
        PrevNumber, Sign,
    },
    bnum::types::{I256, I512, U256, U512},
    std::fmt::Display,
};

/// Describes basic operations that all math types must implement.
pub trait Number: Sized {
    fn checked_add(self, other: Self) -> MathResult<Self>;

    fn checked_sub(self, other: Self) -> MathResult<Self>;

    fn checked_mul(self, other: Self) -> MathResult<Self>;

    fn checked_div(self, other: Self) -> MathResult<Self>;

    fn checked_rem(self, other: Self) -> MathResult<Self>;

    fn checked_pow(self, other: u32) -> MathResult<Self>;

    fn checked_sqrt(self) -> MathResult<Self>;

    fn wrapping_add(self, other: Self) -> Self;

    fn wrapping_sub(self, other: Self) -> Self;

    fn wrapping_mul(self, other: Self) -> Self;

    fn wrapping_pow(self, other: u32) -> Self;

    fn saturating_add(self, other: Self) -> Self;

    fn saturating_sub(self, other: Self) -> Self;

    fn saturating_mul(self, other: Self) -> Self;

    fn saturating_pow(self, other: u32) -> Self;
}

// ------------------------------------ int ------------------------------------

impl<U> Number for Int<U>
where
    U: Number,
{
    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    fn checked_div(self, other: Self) -> MathResult<Self> {
        self.0.checked_div(other.0).map(Self)
    }

    fn checked_rem(self, other: Self) -> MathResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(self, other: u32) -> MathResult<Self> {
        self.0.checked_pow(other).map(Self)
    }

    fn checked_sqrt(self) -> MathResult<Self> {
        self.0.checked_sqrt().map(Self)
    }

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}

// ------------------------------------ dec ------------------------------------

impl<U> Number for Dec<U>
where
    Self: FixedPoint<U> + NumberConst,
    U: NumberConst + Number + IsZero + Copy + PartialEq + PartialOrd + Display,
    Int<U>: NextNumber + Sign,
    <Int<U> as NextNumber>::Next: Number + IsZero + Copy + ToString + PrevNumber<Prev = Int<U>>,
{
    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        let clos = || {
            self.0
                .checked_full_mul(*other.numerator())?
                .checked_div(Self::DECIMAL_FRACTION.into_next())?
                .checked_into_prev()
                .map(Self)
        };

        clos().map_err(|_| MathError::overflow_mul(self, other))
    }

    fn checked_div(self, other: Self) -> MathResult<Self> {
        Dec::checked_from_ratio(*self.numerator(), *other.numerator())
    }

    fn checked_rem(self, other: Self) -> MathResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(mut self, mut exp: u32) -> MathResult<Self> {
        if exp == 0 {
            return Ok(Self::ONE);
        }

        let mut y = Dec::ONE;

        while exp > 1 {
            if exp % 2 == 0 {
                self = self.checked_mul(self)?;
                exp /= 2;
            } else {
                y = self.checked_mul(y)?;
                self = self.checked_mul(self)?;
                exp = (exp - 1) / 2;
            }
        }

        self.checked_mul(y)
    }

    // TODO: Check if this is the best way to implement this
    fn checked_sqrt(self) -> MathResult<Self> {
        // With the current design, U should be only unsigned number.
        // Leave this safety check here for now.
        if self.0 < Int::ZERO {
            return Err(MathError::negative_sqrt::<Self>(self));
        }

        let hundred = Int::TEN.checked_mul(Int::TEN)?;

        (0..=Self::DECIMAL_PLACES / 2)
            .rev()
            .find_map(|i| -> Option<MathResult<Self>> {
                let inner_mul = match hundred.checked_pow(i) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err)),
                };
                self.0.checked_mul(inner_mul).ok().map(|inner| {
                    let outer_mul = Int::TEN.checked_pow(Self::DECIMAL_PLACES / 2 - i)?;
                    Ok(Self::raw(inner.checked_sqrt()?.checked_mul(outer_mul)?))
                })
            })
            .transpose()?
            .ok_or(MathError::SqrtFailed)
    }

    fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    fn saturating_pow(self, other: u32) -> Self {
        Self(self.0.saturating_pow(other))
    }
}

// ------------------------------ primitive types ------------------------------

macro_rules! impl_number {
    ($t:ty) => {
        impl Number for $t
        where
            $t: NumberConst + Integer + IsZero,
        {
            fn checked_add(self, other: Self) -> MathResult<Self> {
                self.checked_add(other)
                    .ok_or_else(|| MathError::overflow_add(self, other))
            }

            fn checked_sub(self, other: Self) -> MathResult<Self> {
                self.checked_sub(other)
                    .ok_or_else(|| MathError::overflow_sub(self, other))
            }

            fn checked_mul(self, other: Self) -> MathResult<Self> {
                self.checked_mul(other)
                    .ok_or_else(|| MathError::overflow_mul(self, other))
            }

            fn checked_div(self, other: Self) -> MathResult<Self> {
                self.checked_div(other)
                    .ok_or_else(|| MathError::division_by_zero(self))
            }

            fn checked_rem(self, other: Self) -> MathResult<Self> {
                self.checked_rem(other)
                    .ok_or_else(|| MathError::division_by_zero(self))
            }

            fn checked_pow(self, other: u32) -> MathResult<Self> {
                self.checked_pow(other)
                    .ok_or_else(|| MathError::overflow_pow(self, other))
            }

            /// Compute a _positive_ integer's _floored_ square root using the
            /// [Babylonian method](https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Heron's_method).
            fn checked_sqrt(self) -> MathResult<Self> {
                if self.is_zero() {
                    return Ok(Self::ZERO);
                }

                if self.is_negative() {
                    return Err(MathError::negative_sqrt(self));
                }

                let mut x0 = Self::ONE << ((Integer::checked_ilog2(self)? / 2) + 1);

                if x0 > Self::ZERO {
                    let mut x1 = (x0 + self / x0) >> 1;

                    while x1 < x0 {
                        x0 = x1;
                        x1 = (x0 + self / x0) >> 1;
                    }

                    return Ok(x0);
                }

                Ok(self)
            }

            fn wrapping_add(self, other: Self) -> Self {
                self.wrapping_add(other)
            }

            fn wrapping_sub(self, other: Self) -> Self {
                self.wrapping_sub(other)
            }

            fn wrapping_mul(self, other: Self) -> Self {
                self.wrapping_mul(other)
            }

            fn wrapping_pow(self, other: u32) -> Self {
                self.wrapping_pow(other)
            }

            fn saturating_add(self, other: Self) -> Self {
                self.saturating_add(other)
            }

            fn saturating_sub(self, other: Self) -> Self {
                self.saturating_sub(other)
            }

            fn saturating_mul(self, other: Self) -> Self {
                self.saturating_mul(other)
            }

            fn saturating_pow(self, other: u32) -> Self {
                self.saturating_pow(other)
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_number!($t);
        )+
    };
}

impl_number! {
    u8, u16, u32, u64, u128, U256, U512,
    i8, i16, i32, i64, i128, I256, I512,
}

// ------------------------------------ tests ------------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{dts, int_test, test_utils::bt, Int, MathError, Number, NumberConst},
        bnum::types::{I256, U256},
    };

    int_test!( checked_add
        inputs = {
            u128 = {
                passing: [
                    (0_u128, 0_u128, 0_u128),
                    (0, u128::MAX, u128::MAX),
                    (10, 20, 30),
                ],
                failing: [
                    (u128::MAX, 1_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::ZERO, U256::MAX, U256::MAX),
                    (U256::from(10_u32), U256::from(20_u32), U256::from(30_u32))
                ],
                failing: [
                    (U256::MAX, U256::ONE),
                ]
            }
            i128 = {
                passing: [
                    (0_i128, 0_i128, 0_i128),
                    (0, i128::MAX, i128::MAX),
                    (0, i128::MIN, i128::MIN),
                    (10, 20, 30),
                    (-10, 20, 10),
                    (10, -20, -10),
                    (-10, -20, -30)
                ],
                failing: [
                    (i128::MAX, 1),
                    (i128::MIN, -1),
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, I256::ZERO, I256::ZERO),
                    (I256::ZERO, I256::MAX, I256::MAX),
                    (I256::ZERO, I256::MIN, I256::MIN),
                    (I256::from(10), I256::from(20), I256::from(30)),
                    (I256::from(-10), I256::from(20), I256::from(10)),
                    (I256::from(10), I256::from(-20), I256::from(-10)),
                    (I256::from(-10), I256::from(-20), I256::from(-30)),
                ]
                failing: [
                    (I256::MAX, I256::ONE),
                    (I256::MIN, -I256::ONE),
                ]
            }
        }
        method = |_0, passing, failing| {
            for (left, right, expected) in passing {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left + right, expected);
            }

            for (left, right) in failing {
                let left = Int::new(left);
                let right = Int::new(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_add(right), Err(MathError::OverflowAdd { .. })));
            }
        }
    );

    int_test!( add_panic
        attrs = #[should_panic(expected = "addition overflow")]
        method = |_0| {
            let max = bt(_0, Int::MAX);
            let one = bt(_0,Int::ONE);
            let _ = max + one;
        }
    );

    int_test!( add_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0| {
            let mut a = bt(_0, Int::new(14_u64.into()));
            a += bt(_0, Int::new(2_u64.into()));
            assert_eq!(a, bt(_0, Int::new(16_u64.into())));
        }
    );

    int_test!( checked_sub
        inputs = {
            u128 = {
                passing: [
                    (0_u128, 0_u128, 0_u128),
                    (u128::MAX, u128::MAX, 0),
                    (30, 20, 10),
                ],
                failing: [
                    (1_u128, 2_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::MAX, U256::MAX, U256::ZERO),
                    (U256::from(30_u32), U256::from(10_u32), U256::from(20_u32)),
                ],
                failing: [
                    (U256::ONE, U256::from(2_u32)),
                ]
            }
            i128 = {
                passing: [
                    (0_i128, 0_i128, 0_i128),
                    (i128::MAX, i128::MAX, 0),
                    (i128::MIN, i128::MIN, 0),
                    (0, i128::MIN + i128::ONE, i128::MAX),
                    (30, 20, 10),
                    (-10, 20, -30),
                    (10, -20, 30),
                    (-10, -20, 10),
                ],
                failing: [
                    (i128::MIN, 1),
                    (i128::MAX, -1),
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, I256::ZERO, I256::ZERO),
                    (I256::MAX, I256::MAX, I256::ZERO),
                    (I256::MIN, I256::MIN, I256::ZERO),
                    (I256::ZERO, I256::MIN + I256::ONE, I256::MAX),
                    (I256::from(30), I256::from(20), I256::from(10)),
                    (I256::from(-10), I256::from(20), I256::from(-30)),
                    (I256::from(10), I256::from(-20), I256::from(30)),
                    (I256::from(-10), I256::from(-20), I256::from(10)),
                ],
                failing: [
                    (I256::MIN, I256::ONE),
                    (I256::MAX, -I256::ONE),
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (left, right, expected) in samples {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left - right, expected);
            }

            for (left, right) in failing_samples {
                let left = Int::new(left);
                let right = Int::new(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_sub(right), Err(MathError::OverflowSub { .. })));
            }
        }
    );

    int_test!( sub_panic
        attrs = #[should_panic(expected = "subtraction overflow")]
        method = |_0| {
            let max = bt(_0, Int::MIN);
            let one = bt(_0, Int::ONE);
            let _ = max - one;
        }
    );

    int_test!( sub_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0| {
            let mut a = bt(_0, Int::new(14_u64.into()));
            a -= bt(_0, Int::new(2_u64.into()));
            assert_eq!(a, bt(_0, Int::new(12_u64.into())));
        }
    );

    int_test!( checked_mul
        inputs = {
            u128 = {
                passing: [
                    (0_u128, 0_u128, 0_u128),
                    (u128::MAX, 0, 0),
                    (30, 20, 600),
                ],
                failing: [
                    (u128::MAX, 2_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::MAX, U256::ZERO, U256::ZERO),
                    (U256::from(30_u32), U256::from(10_u32), U256::from(300_u32)),
                ],
                failing: [
                    (U256::MAX, U256::from(2_u32)),
                ]
            }
            i128 = {
                passing: [
                    (0_i128, 0_i128, 0_i128),
                    (i128::MAX, 0, 0),
                    (i128::MIN, 1, i128::MIN),
                    (i128::MIN + 1, -1, i128::MAX),
                    (i128::MAX, -1, i128::MIN + 1),
                    (30, 20, 600),
                    (-10, 20, -200),
                    (10, -20, -200),
                    (-10, -20, 200),
                ],
                failing: [
                    (i128::MIN, 2),
                    (i128::MIN, -2),
                    (i128::MAX, 2),
                    (i128::MAX, -2),
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, I256::ZERO, I256::ZERO),
                    (I256::MAX, I256::ZERO, I256::ZERO),
                    (I256::MIN, I256::ONE, I256::MIN),
                    (I256::MIN + I256::ONE, -I256::ONE, I256::MAX),
                    (I256::MAX, -I256::ONE, I256::MIN + I256::ONE),
                    (I256::from(30), I256::from(20), I256::from(600)),
                    (I256::from(-10), I256::from(20), I256::from(-200)),
                    (I256::from(10), I256::from(-20), I256::from(-200)),
                    (I256::from(-10), I256::from(-20), I256::from(200)),
                ],
                failing: [
                    (I256::MIN, I256::from(2)),
                    (I256::MIN, I256::from(-2)),
                    (I256::MAX, I256::from(2)),
                    (I256::MAX, I256::from(-2)),
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (left, right, expected) in samples {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left * right, expected);
            }

            for (left, right) in failing_samples {
                let left = Int::new(left);
                let right = Int::new(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_mul(right), Err(MathError::OverflowMul { .. })));
            }
        }
    );

    int_test!( mul_panic
        attrs = #[should_panic(expected = "multiplication overflow")]
        method = |_0| {
            let max = bt(_0, Int::MAX);
            let one = bt(_0, Int::new(2_u64.into()));
            let _ = max * one;
        }
    );

    int_test!( mul_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0| {
            let mut a = bt(_0, Int::new(14_u64.into()));
            a *= bt(_0, Int::new(2_u64.into()));
            assert_eq!(a, bt(_0, Int::new(28_u64.into())));
        }
    );

    int_test!( checked_div
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 1_u128, u128::MAX),
                    (0, 1, 0),
                    (300, 20, 15),
                    (30, 20, 1),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, U256::ONE, U256::MAX),
                    (U256::ZERO, U256::ONE, U256::ZERO),
                    (U256::from(300_u32), U256::from(20_u32), U256::from(15_u32)),
                    (U256::from(30_u32), U256::from(20_u32), U256::from(1_u32)),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 1_i128, i128::MAX),
                    (i128::MIN, 1_i128, i128::MIN),
                    (i128::MIN + 1, -1_i128, i128::MAX),
                    (i128::MAX , -1_i128, i128::MIN + 1),
                    (300, 20, 15),
                    (30, 20, 1),
                    (-300, 20, -15),
                    (-30, 20, -1),
                    (-300, -20, 15),
                    (300, -20, -15),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, I256::ONE, I256::MAX),
                    (I256::MIN, I256::ONE, I256::MIN),
                    (I256::MIN + I256::ONE, -I256::ONE, I256::MAX),
                    (I256::MAX, -I256::ONE, I256::MIN + I256::ONE),
                    (I256::from(300), I256::from(20), I256::from(15)),
                    (I256::from(30), I256::from(20), I256::from(1)),
                    (I256::from(-300), I256::from(20), I256::from(-15)),
                    (I256::from(-30), I256::from(20), I256::from(-1)),
                    (I256::from(-300), I256::from(-20), I256::from(15)),
                    (I256::from(300), I256::from(-20), I256::from(-15)),
                ]
            }
        }
        method = |_0, samples| {
            for (left, right, expected) in samples {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left / right, expected);
            }

            // Division by zero
            let zero = Int::ZERO;
            let one = Int::ONE;
            dts!(_0, one, zero);
            assert!(matches!(one.checked_div(zero), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( div_panic
        attrs = #[should_panic(expected = "division by zero")]
        method = |_0| {
            let max = bt(_0, Int::MAX);
            let _ = max / _0;
        }
    );

    int_test!( div_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0| {
            let mut a = bt(_0, Int::new(14_u64.into()));
            a /= bt(_0, Int::new(2_u64.into()));
            assert_eq!(a, bt(_0, Int::new(7_u64.into())));
        }
    );

    int_test!( checked_pow
        inputs = {
            u128 = {
                passing: [
                    (2_u128, 2, 4_u128),
                    (10, 3, 1_000),
                    (0, 2, 0),
                ],
                failing: [
                    (u128::MAX, 2),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(2_u32), 2, U256::from(4_u32)),
                    (U256::from(10_u32), 3, U256::from(1_000_u32)),
                    (U256::ZERO, 2, U256::ZERO),
                ],
                failing: [
                    (U256::MAX, 2),
                ]
            }
            i128 = {
                passing: [
                    (2_i128, 2, 4_i128),
                    (10, 3, 1_000),
                    (-2, 2, 4),
                    (-10, 3, -1_000),
                    (0, 2, 0),
                ],
                failing: [
                    (i128::MAX, 2),
                    (i128::MIN, 2),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(2), 2, I256::from(4)),
                    (I256::from(10), 3, I256::from(1_000)),
                    (I256::from(-2), 2, I256::from(4)),
                    (I256::from(-10), 3, I256::from(-1_000)),
                    (I256::ZERO, 2, I256::ZERO),
                ],
                failing: [
                    (I256::MAX, 2),
                    (I256::MIN, 2),
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (base, exp, expected) in samples {
                let base = Int::new(base);
                let expected = Int::new(expected);
                dts!(_0, base, expected);
                assert_eq!(base.checked_pow(exp).unwrap(), expected);
            }

            for (base, exp) in failing_samples {
                let base = bt(_0, Int::new(base));
                assert!(matches!(base.checked_pow(exp), Err(MathError::OverflowPow { .. })));
            }
        }
    );

    int_test!( checked_sqrt
        inputs = {
            u128 = {
                passing: [
                    (4_u128, 2_u128),
                    (64, 8),
                    (80, 8),
                    (81, 9),
                ],
                failing: []
            }
            u256 = {
                passing: [
                    (U256::from(4_u32), U256::from(2_u32)),
                    (U256::from(64_u32), U256::from(8_u32)),
                    (U256::from(80_u32), U256::from(8_u32)),
                    (U256::from(81_u32), U256::from(9_u32)),
                ],
                failing: []
            }
            i128 = {
                passing: [
                    (4_i128, 2_i128),
                    (64, 8),
                    (80, 8),
                    (81, 9),
                ],
                failing: [
                    -1_i128,
                    -4_i128,
                ]
            }
            i256 = {
                passing: [
                    (I256::from(4_i128), I256::from(2_i128)),
                    (I256::from(64), I256::from(8)),
                    (I256::from(80), I256::from(8)),
                    (I256::from(81), I256::from(9)),
                ],
                failing: [
                    I256::from(-1),
                    I256::from(-4),
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (base, expected) in samples {
                let base = Int::new(base);
                let expected = Int::new(expected);
                dts!(_0, base, expected);
                assert_eq!(base.checked_sqrt().unwrap(), expected);
            }

            for base in failing_samples {
                let base = bt(_0, Int::new(base));
                // base.checked_sqrt().unwrap();
                assert!(matches!(base.checked_sqrt(), Err(MathError::NegativeSqrt { .. })));
            }
        }
    );

    int_test!( checked_rem
        inputs = {
            u128 = {
                passing: [
                    (10_u128, 4_u128, 2_u128),
                    (10_u128, 3_u128, 1_u128),
                    (10_u128, 1_u128, 0_u128),
                    (10_u128, 2_u128, 0_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u32), U256::from(4_u32), U256::from(2_u32)),
                    (U256::from(10_u32), U256::from(3_u32), U256::from(1_u32)),
                    (U256::from(10_u32), U256::ONE, U256::ZERO),
                    (U256::from(10_u32), U256::from(2_u32), U256::ZERO),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, 4_i128, 2_i128),
                    (10_i128, 3_i128, 1_i128),
                    (10_i128, 1_i128, 0_i128),
                    (10_i128, 2_i128, 0_i128),
                    (-10_i128, 4_i128, -2_i128),
                    (-10_i128, 3_i128, -1_i128),
                    (-10_i128, 1_i128, 0_i128),
                    (-10_i128, 2_i128, 0_i128),
                    (10_i128, -4_i128, 2_i128),
                    (10_i128, -3_i128, 1_i128),
                    (10_i128, -1_i128, 0_i128),
                    (10_i128, -2_i128, 0_i128),
                    (-10_i128, -4_i128, -2_i128),
                    (-10_i128, -3_i128, -1_i128),
                    (-10_i128, -1_i128, 0_i128),
                    (-10_i128, -2_i128, 0_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), I256::from(4_i128), I256::from(2_i128)),
                    (I256::from(10_i128), I256::from(3_i128), I256::from(1_i128)),
                    (I256::from(10_i128), I256::ONE, I256::ZERO),
                    (I256::from(10_i128), I256::from(2_i128), I256::ZERO),
                    (I256::from(-10_i128), I256::from(4_i128), I256::from(-2_i128)),
                    (I256::from(-10_i128), I256::from(3_i128), I256::from(-1_i128)),
                    (I256::from(-10_i128), I256::ONE, I256::ZERO),
                    (I256::from(-10_i128), I256::from(2_i128), I256::ZERO),
                    (I256::from(10_i128), I256::from(-4_i128), I256::from(2_i128)),
                    (I256::from(10_i128), I256::from(-3_i128), I256::from(1_i128)),
                    (I256::from(10_i128), I256::from(-1_i128), I256::ZERO),
                    (I256::from(10_i128), I256::from(-2_i128), I256::ZERO),
                    (I256::from(-10_i128), I256::from(-4_i128), I256::from(-2_i128)),
                    (I256::from(-10_i128), I256::from(-3_i128), I256::from(-1_i128)),
                    (I256::from(-10_i128), I256::from(-1_i128), I256::ZERO),
                    (I256::from(-10_i128), I256::from(-2_i128), I256::ZERO),
                ]
            }
        }
        method = |_0, passing| {
            for (base, div, expected) in passing {
                let base = Int::new(base);
                let div = Int::new(div);
                let expected = Int::new(expected);
                dts!(_0, base, div, expected);
                assert_eq!(base.checked_rem(div).unwrap(), expected);
            }

            // Division by zero
            let ten = Int::TEN;
            assert!(matches!(ten.checked_rem(_0), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( rem_panic
        attrs = #[should_panic(expected = "division by zero")]
        method = |_0| {
            let max = bt(_0, Int::MAX);
            let _ = max % _0;
        }
    );

    int_test!( rem_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0| {
            let mut a = bt(_0, Int::new(14_u64.into()));
            a %= bt(_0, Int::new(3_u64.into()));
            assert_eq!(a, bt(_0, Int::new(2_u64.into())));
        }
    );

    int_test!( wrapping_add
        method = |_0| {
            let max = bt(_0, Int::MAX);
            assert_eq!(max.wrapping_add(Int::ONE), Int::MIN);
        }
    );

    int_test!( wrapping_sub
        method = |_0| {
            let min = bt(_0, Int::MIN);
            assert_eq!(min.wrapping_sub(Int::ONE), Int::MAX);
        }
    );

    int_test!( wrapping_mul
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 2_u128, u128::MAX - 1),
                    (u128::MAX, 3_u128, u128::MAX - 2),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, U256::from(2_u32), U256::MAX - U256::ONE),
                    (U256::MAX, U256::from(3_u32), U256::MAX - U256::from(2_u32)),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 2_i128, -2_i128),
                    (i128::MAX, 3_i128, i128::MAX - 2),
                    (i128::MAX, 4_i128, -4_i128),
                    (i128::MAX, 5_i128, i128::MAX - 4),
                    (i128::MIN, 2_i128, 0),
                    (i128::MIN, 3_i128, i128::MIN),
                    (i128::MIN, 4_i128, 0),
                    (i128::MIN, 5_i128, i128::MIN),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, I256::from(2), I256::from(-2)),
                    (I256::MAX, I256::from(3), I256::MAX - I256::from(2)),
                    (I256::MAX, I256::from(4), I256::from(-4)),
                    (I256::MAX, I256::from(5), I256::MAX - I256::from(4)),
                    (I256::MIN, I256::from(2), I256::ZERO),
                    (I256::MIN, I256::from(3), I256::MIN),
                    (I256::MIN, I256::from(4), I256::ZERO),
                    (I256::MIN, I256::from(5), I256::MIN),
                ]
            }
        }
        method = |_0, samples| {
            for (left, right, expected) in samples {
                let left = Int::new(left);
                let right = Int::new(right);
                let expected = Int::new(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left.wrapping_mul(right), expected);
            }
       }
    );

    int_test!( wrapping_pow
        inputs = {
            u128 = {
                passing: [
                    (u128::MAX, 2, 1),
                    (u128::MAX, 3, u128::MAX),
                ]
            }
            u256 = {
                passing: [
                    (U256::MAX, 2, U256::ONE),
                    (U256::MAX, 3, U256::MAX),
                ]
            }
            i128 = {
                passing: [
                    (i128::MAX, 2, 1),
                    (i128::MAX, 3, i128::MAX),
                    (i128::MAX, 4, 1),
                    (i128::MAX, 5, i128::MAX),
                    (i128::MIN, 2, 0),
                    (i128::MIN, 3, 0),
                    (i128::MIN, 4, 0),
                ]
            }
            i256 = {
                passing: [
                    (I256::MAX, 2, I256::ONE),
                    (I256::MAX, 3, I256::MAX),
                    (I256::MAX, 4, I256::ONE),
                    (I256::MAX, 5, I256::MAX),
                    (I256::MIN, 2, I256::ZERO),
                    (I256::MIN, 3, I256::ZERO),
                    (I256::MIN, 4, I256::ZERO),
                ]
            }
        }
        method = |_0, samples| {
            for (base, exp, expected) in samples {
                let base = Int::new(base);
                let expected = Int::new(expected);
                dts!(_0, base, expected);
                assert_eq!(base.wrapping_pow(exp), expected);
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use crate::{
        dec_test, dts,
        test_utils::{bt, dec},
        Dec, Int, MathError, Number, NumberConst,
    };

    dec_test!( checked_add
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::ZERO, Dec::MAX, Dec::MAX),
                    (dec("10"), dec("20"), dec("30")),
                    (dec("0.1"), dec("20"), dec("20.1")),
                    (dec("0.01"), dec("20"), dec("20.01")),
                    (dec("0.001"), dec("20"), dec("20.001")),
                    (dec("0.0001"), dec("20"), dec("20.0001")),
                ],
                failing: [
                    (Dec::MAX, Dec::ONE),
                ]
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::ZERO, Dec::MAX, Dec::MAX),
                    (dec("10"), dec("20"), dec("30")),
                    (dec("0.1"), dec("20"), dec("20.1")),
                    (dec("0.01"), dec("20"), dec("20.01")),
                    (dec("0.001"), dec("20"), dec("20.001")),
                    (dec("0.0001"), dec("20"), dec("20.0001")),
                ],
                failing: [
                    (Dec::MAX, Dec::ONE),
                ]
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::ZERO, Dec::MAX, Dec::MAX),
                    (dec("10"), dec("20"), dec("30")),
                    (dec("0.1"), dec("20"), dec("20.1")),
                    (dec("0.01"), dec("20"), dec("20.01")),
                    (dec("0.001"), dec("20"), dec("20.001")),
                    (dec("0.0001"), dec("20"), dec("20.0001")),

                    (dec("-10"), dec("20"), dec("10")),
                    (dec("10"), dec("-20"), dec("-10")),
                    (dec("-10"), dec("-20"), dec("-30")),
                    (dec("-0.1"), dec("20"), dec("19.9")),
                    (dec("-0.01"), dec("20"), dec("19.99")),
                    (dec("0.1"), dec("-20"), dec("-19.9")),
                    (dec("0.01"), dec("-20"), dec("-19.99")),
                    (dec("-0.1"), dec("-20"), dec("-20.1")),
                    (dec("-0.01"), dec("-20"), dec("-20.01")),
                ],
                failing: [
                    (Dec::MAX, Dec::ONE),
                    (Dec::MIN, -Dec::ONE),
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::ZERO, Dec::MAX, Dec::MAX),
                    (dec("10"), dec("20"), dec("30")),
                    (dec("0.1"), dec("20"), dec("20.1")),
                    (dec("0.01"), dec("20"), dec("20.01")),
                    (dec("0.001"), dec("20"), dec("20.001")),
                    (dec("0.0001"), dec("20"), dec("20.0001")),

                    (dec("-10"), dec("20"), dec("10")),
                    (dec("10"), dec("-20"), dec("-10")),
                    (dec("-10"), dec("-20"), dec("-30")),
                    (dec("-0.1"), dec("20"), dec("19.9")),
                    (dec("-0.01"), dec("20"), dec("19.99")),
                    (dec("0.1"), dec("-20"), dec("-19.9")),
                    (dec("0.01"), dec("-20"), dec("-19.99")),
                    (dec("-0.1"), dec("-20"), dec("-20.1")),
                    (dec("-0.01"), dec("-20"), dec("-20.01")),
                ],
                failing: [
                    (Dec::MAX, Dec::ONE),
                    (Dec::MIN, -Dec::ONE),
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.checked_add(right).unwrap(), expected);
            }

            for (left, right) in failing {
                dts!(_0d, left, right);
                assert!(matches!(left.checked_add(right), Err(MathError::OverflowAdd { .. })));
            }
        }
    );

    dec_test!( add_panic
        attrs = #[should_panic(expected = "addition overflow")]
        method = |_0d| {
            let max = bt(_0d, Dec::MAX);
            let one = bt(_0d,Dec::ONE);
            let _ = max + one;
        }
    );

    dec_test!( add_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0d| {
            let mut a = bt(_0d, dec("14"));
            a += bt(_0d, dec("2.5"));
            assert_eq!(a, bt(_0d, dec("16.5")));
        }
    );

    dec_test!( checked_sub
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::MAX),
                    (dec("20"), dec("10"), dec("10")),
                    (dec("20"), dec("0.1"), dec("19.9")),
                    (dec("20"), dec("0.01"), dec("19.99")),
                    (dec("20"), dec("0.001"), dec("19.999")),
                    (dec("20"), dec("0.0001"), dec("19.9999")),
                ],
                failing: [
                    (Dec::ZERO, Dec::ONE),
                ]
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::MAX),
                    (dec("20"), dec("10"), dec("10")),
                    (dec("20"), dec("0.1"), dec("19.9")),
                    (dec("20"), dec("0.01"), dec("19.99")),
                    (dec("20"), dec("0.001"), dec("19.999")),
                    (dec("20"), dec("0.0001"), dec("19.9999")),
                ],
                failing: [
                    (Dec::ZERO, Dec::ONE),
                ]
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::MAX),
                    (Dec::MIN, Dec::ZERO, Dec::MIN),
                    (dec("20"), dec("10"), dec("10")),
                    (dec("20"), dec("0.1"), dec("19.9")),
                    (dec("20"), dec("0.01"), dec("19.99")),
                    (dec("20"), dec("0.001"), dec("19.999")),
                    (dec("20"), dec("0.0001"), dec("19.9999")),

                    (dec("-20"), dec("10"), dec("-30")),
                    (dec("20"), dec("-10"), dec("30")),
                    (dec("-20"), dec("-10"), dec("-10")),
                    (dec("20"), dec("-0.1"), dec("20.1")),
                    (dec("20"), dec("-0.01"), dec("20.01")),
                    (dec("-20"), dec("0.1"), dec("-20.1")),
                    (dec("-20"), dec("0.01"), dec("-20.01")),
                    (dec("-20"), dec("-0.1"), dec("-19.9")),
                    (dec("-20"), dec("-0.01"), dec("-19.99")),
                ],
                failing: [
                    (Dec::MAX, -Dec::ONE),
                    (Dec::MIN, Dec::ONE),
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::MAX),
                    (Dec::MIN, Dec::ZERO, Dec::MIN),
                    (dec("20"), dec("10"), dec("10")),
                    (dec("20"), dec("0.1"), dec("19.9")),
                    (dec("20"), dec("0.01"), dec("19.99")),
                    (dec("20"), dec("0.001"), dec("19.999")),
                    (dec("20"), dec("0.0001"), dec("19.9999")),

                    (dec("-20"), dec("10"), dec("-30")),
                    (dec("20"), dec("-10"), dec("30")),
                    (dec("-20"), dec("-10"), dec("-10")),
                    (dec("20"), dec("-0.1"), dec("20.1")),
                    (dec("20"), dec("-0.01"), dec("20.01")),
                    (dec("-20"), dec("0.1"), dec("-20.1")),
                    (dec("-20"), dec("0.01"), dec("-20.01")),
                    (dec("-20"), dec("-0.1"), dec("-19.9")),
                    (dec("-20"), dec("-0.01"), dec("-19.99")),
                ],
                failing: [
                    (Dec::MAX, -Dec::ONE),
                    (Dec::MIN, Dec::ONE),
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.checked_sub(right).unwrap(), expected);
            }

            for (left, right) in failing {
                dts!(_0d, left, right);
                assert!(matches!(left.checked_sub(right), Err(MathError::OverflowSub { .. })));
            }
        }
    );

    dec_test!( sub_panic
        attrs = #[should_panic(expected = "subtraction overflow")]
        method = |_0d| {
            let max = bt(_0d, Dec::MIN);
            let one = bt(_0d, Dec::ONE);
            let _ = max - one;
        }
    );

    dec_test!( sub_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0d| {
            let mut a = bt(_0d, dec("14"));
            a -= bt(_0d, dec("2.5"));
            assert_eq!(a, bt(_0d, dec("11.5")));
        }
    );

    dec_test!( checked_mul
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::ZERO),
                    (dec("20"), dec("10"), dec("200")),
                    (dec("20"), dec("1.5"), dec("30")),
                    (dec("20"), dec("0.1"), dec("2")),
                    (dec("20"), dec("0.01"), dec("0.2")),
                    (dec("20"), dec("0.001"), dec("0.02")),
                    (dec("20"), dec("0.0001"), dec("0.002")),
                ],
                failing: [
                    (Dec::MAX, dec("2")),
                ]
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::ZERO),
                    (dec("20"), dec("10"), dec("200")),
                    (dec("20"), dec("1.5"), dec("30")),
                    (dec("20"), dec("0.1"), dec("2")),
                    (dec("20"), dec("0.01"), dec("0.2")),
                    (dec("20"), dec("0.001"), dec("0.02")),
                    (dec("20"), dec("0.0001"), dec("0.002")),
                ],
                failing: [
                    (Dec::MAX, dec("2")),
                ]
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::ZERO),
                    (Dec::MIN + Dec::raw(Int::new(1)), -Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("200")),
                    (dec("20"), dec("1.5"), dec("30")),
                    (dec("20"), dec("0.1"), dec("2")),
                    (dec("20"), dec("0.01"), dec("0.2")),
                    (dec("20"), dec("0.001"), dec("0.02")),
                    (dec("20"), dec("0.0001"), dec("0.002")),

                    (dec("-20"), dec("1.5"), dec("-30")),
                    (dec("20"), dec("-1.5"), dec("-30")),
                    (dec("-20"), dec("-1.5"), dec("30")),
                ],
                failing: [
                    (Dec::MAX, dec("2")),
                    (Dec::MIN, dec("2")),
                    (Dec::MIN, -Dec::ONE),
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, Dec::ZERO, Dec::ZERO),
                    (Dec::MIN + Dec::raw(Int::new(1.into())), -Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("200")),
                    (dec("20"), dec("1.5"), dec("30")),
                    (dec("20"), dec("0.1"), dec("2")),
                    (dec("20"), dec("0.01"), dec("0.2")),
                    (dec("20"), dec("0.001"), dec("0.02")),
                    (dec("20"), dec("0.0001"), dec("0.002")),

                    (dec("-20"), dec("1.5"), dec("-30")),
                    (dec("20"), dec("-1.5"), dec("-30")),
                    (dec("-20"), dec("-1.5"), dec("30")),
                ],
                failing: [
                    (Dec::MAX, dec("2")),
                    (Dec::MIN, dec("2")),
                    (Dec::MIN, -Dec::ONE),
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.checked_mul(right).unwrap(), expected);
            }

            for (left, right) in failing {
                dts!(_0d, left, right);
                assert!(matches!(left.checked_mul(right), Err(MathError::OverflowMul { .. })));
            }
        }
    );

    dec_test!( mul_panic
        attrs = #[should_panic(expected = "multiplication overflow")]
        method = |_0d| {
            let max = bt(_0d, Dec::MAX);
            let one = bt(_0d, dec("2"));
            let _ = max * one;
        }
    );

    dec_test!( mul_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0d| {
            let mut a = bt(_0d, dec("14"));
            a *= bt(_0d, dec("2.5"));
            assert_eq!(a, bt(_0d, dec("35")));
        }
    );
}
