use {
    crate::{
        Dec, FixedPoint, Fraction, Int, Integer, IsZero, MathError, MathResult, MultiplyRatio,
        NextNumber, NumberConst, PrevNumber, Sign,
    },
    bnum::types::{I256, I512, U256, U512},
    std::fmt::Display,
};

/// Describes basic operations that all math types must implement.
pub trait Number: Sized + Copy {
    fn checked_add(self, other: Self) -> MathResult<Self>;

    fn checked_sub(self, other: Self) -> MathResult<Self>;

    fn checked_mul(self, other: Self) -> MathResult<Self>;

    fn checked_div(self, other: Self) -> MathResult<Self>;

    fn checked_rem(self, other: Self) -> MathResult<Self>;

    fn checked_pow(self, exponent: u32) -> MathResult<Self>;

    fn checked_sqrt(self) -> MathResult<Self>;

    #[inline]
    fn checked_add_assign(&mut self, other: Self) -> MathResult<()> {
        *self = self.checked_add(other)?;
        Ok(())
    }

    #[inline]
    fn checked_sub_assign(&mut self, other: Self) -> MathResult<()> {
        *self = self.checked_sub(other)?;
        Ok(())
    }

    #[inline]
    fn checked_mul_assign(&mut self, other: Self) -> MathResult<()> {
        *self = self.checked_mul(other)?;
        Ok(())
    }

    #[inline]
    fn checked_div_assign(&mut self, other: Self) -> MathResult<()> {
        *self = self.checked_div(other)?;
        Ok(())
    }

    #[inline]
    fn checked_rem_assign(&mut self, other: Self) -> MathResult<()> {
        *self = self.checked_rem(other)?;
        Ok(())
    }

    #[inline]
    fn checked_pow_assign(&mut self, exp: u32) -> MathResult<()> {
        *self = self.checked_pow(exp)?;
        Ok(())
    }

    #[inline]
    fn checked_sqrt_assign(&mut self) -> MathResult<()> {
        *self = self.checked_sqrt()?;
        Ok(())
    }

    fn saturating_add(self, other: Self) -> Self;

    fn saturating_sub(self, other: Self) -> Self;

    fn saturating_mul(self, other: Self) -> Self;

    fn saturating_pow(self, exp: u32) -> Self;
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

    fn checked_pow(self, exp: u32) -> MathResult<Self> {
        self.0.checked_pow(exp).map(Self)
    }

    fn checked_sqrt(self) -> MathResult<Self> {
        self.0.checked_sqrt().map(Self)
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

    fn saturating_pow(self, exp: u32) -> Self {
        Self(self.0.saturating_pow(exp))
    }
}

// ------------------------------------ dec ------------------------------------

impl<U> Number for Dec<U>
where
    Self: FixedPoint<U> + NumberConst + Sign,
    U: NumberConst + Number + IsZero + Copy + PartialEq + PartialOrd + Display,
    Int<U>: NextNumber + Sign + MultiplyRatio,
    <Int<U> as NextNumber>::Next: Number + PrevNumber<Prev = Int<U>>,
{
    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        (|| {
            self.0
                .checked_full_mul(other.numerator())?
                .checked_div(Self::PRECISION.into_next())?
                .checked_into_prev()
                .map(Self)
        })()
        .map_err(|_| MathError::overflow_mul(self, other))
    }

    fn checked_div(self, other: Self) -> MathResult<Self> {
        Dec::checked_from_ratio(self.numerator(), other.numerator())
    }

    fn checked_rem(self, other: Self) -> MathResult<Self> {
        self.0.checked_rem(other.0).map(Self)
    }

    fn checked_pow(mut self, mut exp: u32) -> MathResult<Self> {
        (|| {
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
        })()
        .map_err(|_| MathError::overflow_pow(self, exp))
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

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_mul(self, other: Self) -> Self {
        self.checked_mul(other).unwrap_or_else(|_| {
            if self.is_negative() == other.is_negative() {
                Self::MAX
            } else {
                Self::MIN
            }
        })
    }

    fn saturating_pow(self, exp: u32) -> Self {
        self.checked_pow(exp).unwrap_or_else(|_| {
            if self.is_negative() && exp % 2 == 1 {
                Self::MIN
            } else {
                Self::MAX
            }
        })
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

            fn checked_pow(self, exp: u32) -> MathResult<Self> {
                self.checked_pow(exp)
                    .ok_or_else(|| MathError::overflow_pow(self, exp))
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
        crate::{
            dts, int_test,
            test_utils::{bt, int},
            Int, MathError, Number, NumberConst,
        },
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

    int_test!( saturating_add
        inputs = {
            u128 = {
                passing: [
                    (Int::MAX - Int::ONE, Int::ONE, Int::MAX),
                    (Int::MAX, Int::ONE, Int::MAX),
                ]
            }
            u256 = {
                passing: [
                    (Int::MAX - Int::ONE, Int::ONE, Int::MAX),
                    (Int::MAX, Int::ONE, Int::MAX),
                ]
            }
            i128 = {
                passing: [
                    (Int::MAX - Int::ONE, Int::ONE, Int::MAX),
                    (Int::MAX, Int::ONE, Int::MAX),
                    (Int::MIN, -Int::ONE, Int::MIN),
                ]
            }
            i256 = {
                passing: [
                    (Int::MAX - Int::ONE, Int::ONE, Int::MAX),
                    (Int::MAX, Int::ONE, Int::MAX),
                    (Int::MIN, -Int::ONE, Int::MIN),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_add(right), expected);
            }
        }
    );

    int_test!( saturating_sub
        inputs = {
            u128 = {
                passing: [
                    (Int::MIN + Int::ONE, Int::ONE, Int::MIN),
                    (Int::MIN, Int::ONE, Int::MIN),
                ]
            }
            u256 = {
                passing: [
                    (Int::MIN + Int::ONE, Int::ONE, Int::MIN),
                    (Int::MIN, Int::ONE, Int::MIN),
                ]
            }
            i128 = {
                passing: [
                    (Int::MIN + Int::ONE, Int::ONE, Int::MIN),
                    (Int::MIN, Int::ONE, Int::MIN),
                    (Int::MAX, -Int::ONE, Int::MAX),
                ]
            }
            i256 = {
                passing: [
                    (Int::MIN + Int::ONE, Int::ONE, Int::MIN),
                    (Int::MIN, Int::ONE, Int::MIN),
                    (Int::MAX, -Int::ONE, Int::MAX),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_sub(right), expected);
            }
        }
    );

    int_test!( saturating_mul
        inputs = {
            u128 = {
                passing: [
                    (Int::MAX, int("2"), Int::MAX),
                    (Int::MAX / int("2") + Int::ONE, int("2"), Int::MAX),
                ]
            }
            u256 = {
                passing: [
                    (Int::MAX, int("2"), Int::MAX),
                    (Int::MAX / int("2") + Int::ONE, int("2"), Int::MAX),
                ]
            }
            i128 = {
                passing: [
                    (Int::MAX, int("2"), Int::MAX),
                    (Int::MAX / int("2") + Int::ONE, int("2"), Int::MAX),
                    (Int::MIN , int("2"), Int::MIN),
                    (Int::MIN / int("2") - Int::ONE, int("2"), Int::MIN),
                ]
            }
            i256 = {
                passing: [
                    (Int::MAX, int("2"), Int::MAX),
                    (Int::MAX / int("2") + Int::ONE, int("2"), Int::MAX),
                    (Int::MIN , int("2"), Int::MIN),
                    (Int::MIN / int("2") - Int::ONE, int("2"), Int::MIN),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_mul(right), expected);
            }
        }
    );

    int_test!( saturating_pow
        inputs = {
            u128 = {
                passing: [
                    (int("2"), 2, int("4")),
                    (Int::MAX, 2, Int::MAX),
                    (Int::MAX, 3, Int::MAX),
                ]
            }
            u256 = {
                passing: [
                    (int("2"), 2, int("4")),
                    (Int::MAX, 2, Int::MAX),
                    (Int::MAX, 3, Int::MAX),
                ]
            }
            i128 = {
                passing: [
                    (int("2"), 2, int("4")),
                    (int("-2"), 2, int("4")),
                    (int("-2"), 3, int("-8")),
                    (Int::MAX, 2, Int::MAX),
                    (Int::MAX, 3, Int::MAX),
                    (Int::MIN, 2, Int::MAX),
                    (Int::MIN, 3, Int::MIN),

                ]
            }
            i256 = {
                passing: [
                    (int("2"), 2, int("4")),
                    (int("-2"), 2, int("4")),
                    (int("-2"), 3, int("-8")),
                    (Int::MAX, 2, Int::MAX),
                    (Int::MAX, 3, Int::MAX),
                    (Int::MIN, 2, Int::MAX),
                    (Int::MIN, 3, Int::MIN),
                ]
            }
        }
        method = |_0d: Int<_>, passing| {
            for (base, exp, expected) in passing {
                dts!(_0d, base, expected);
                assert_eq!(base.saturating_pow(exp), expected);
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use crate::{
        dec_test, dts,
        test_utils::{bt, dec},
        Dec, FixedPoint, MathError, Number, NumberConst,
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
            let min = bt(_0d, Dec::MIN);
            let one = bt(_0d, Dec::ONE);
            let _ = min - one;
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
                    (Dec::MIN + Dec::TICK, -Dec::ONE, Dec::MAX),
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
                    (Dec::MIN + Dec::TICK, -Dec::ONE, Dec::MAX),
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

    dec_test!( checked_div
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("2")),
                    (dec("20"), dec("1.5"), dec("13.333333333333333333")),
                    (dec("20"), dec("0.1"), dec("200")),
                    (dec("20"), dec("0.01"), dec("2000")),
                    (dec("20"), dec("100"), dec("0.2")),
                    (dec("2"), dec("8"), dec("0.25")),
                ],
                failing: []
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("2")),
                    (dec("20"), dec("1.5"), dec("13.333333333333333333")),
                    (dec("20"), dec("0.1"), dec("200")),
                    (dec("20"), dec("0.01"), dec("2000")),
                    (dec("20"), dec("100"), dec("0.2")),
                    (dec("2"), dec("8"), dec("0.25")),
                ],
                failing: []
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("2")),
                    (dec("20"), dec("1.5"), dec("13.333333333333333333")),
                    (dec("20"), dec("0.1"), dec("200")),
                    (dec("20"), dec("0.01"), dec("2000")),
                    (dec("20"), dec("100"), dec("0.2")),
                    (dec("2"), dec("8"), dec("0.25")),

                    (Dec::MIN, Dec::ONE, Dec::MIN),
                    (Dec::MIN + Dec::TICK, -Dec::ONE, Dec::MAX),
                    (Dec::MAX , -Dec::ONE, Dec::MIN + Dec::TICK),
                    (dec("20"), dec("-10"), dec("-2")),
                    (dec("-20"), dec("10"), dec("-2")),
                    (dec("-20"), dec("-10"), dec("2")),
                    (dec("20"), dec("-1.5"), dec("-13.333333333333333333")),
                    (dec("20"), dec("-0.1"), dec("-200")),
                    (dec("-20"), dec("0.01"), dec("-2000")),
                    (dec("-2"), dec("-8"), dec("0.25")),
                ],
                failing: [
                    (Dec::MIN, -Dec::ONE),
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                    (dec("20"), dec("10"), dec("2")),
                    (dec("20"), dec("1.5"), dec("13.333333333333333333")),
                    (dec("20"), dec("0.1"), dec("200")),
                    (dec("20"), dec("0.01"), dec("2000")),
                    (dec("20"), dec("100"), dec("0.2")),
                    (dec("2"), dec("8"), dec("0.25")),

                    (Dec::MIN, Dec::ONE, Dec::MIN),
                    (Dec::MIN + Dec::TICK, -Dec::ONE, Dec::MAX),
                    (Dec::MAX , -Dec::ONE, Dec::MIN + Dec::TICK),
                    (dec("20"), dec("-10"), dec("-2")),
                    (dec("-20"), dec("10"), dec("-2")),
                    (dec("-20"), dec("-10"), dec("2")),
                    (dec("20"), dec("-1.5"), dec("-13.333333333333333333")),
                    (dec("20"), dec("-0.1"), dec("-200")),
                    (dec("-20"), dec("0.01"), dec("-2000")),
                    (dec("-2"), dec("-8"), dec("0.25")),
                ],
                failing: [
                    (Dec::MIN, -Dec::ONE),
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.checked_div(right).unwrap(), expected);
            }

            for (left, right) in failing {
                dts!(_0d, left, right);
                assert!(matches!(left.checked_div(right), Err(MathError::OverflowConversion { .. })));
            }

            // Division by zero
            let ten = bt(_0d, Dec::TEN);
            assert!(matches!(ten.checked_div(_0d), Err(MathError::DivisionByZero { .. })));
        }
    );

    dec_test!( div_panic
        attrs = #[should_panic(expected = "division by zero")]
        method = |_0d| {
            let max = bt(_0d, Dec::MAX);
            let _ = max / _0d;
        }
    );

    dec_test!( div_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0d| {
            let mut a = bt(_0d, dec("10"));
            a /= bt(_0d, dec("2.5"));
            assert_eq!(a, dec("4"));
        }
    );

    dec_test!( checked_pow
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, 2, Dec::ZERO),
                    (dec("10"), 2, dec("100")),
                    (dec("2.5"), 2, dec("6.25")),
                    (dec("123.123"), 3, dec("1866455.185461867")),
                ],
                failing: [
                    (Dec::MAX, 2),
                ]
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, 2, Dec::ZERO),
                    (dec("10"), 2, dec("100")),
                    (dec("2.5"), 2, dec("6.25")),
                    (dec("123.123"), 3, dec("1866455.185461867")),
                ],
                failing: [
                    (Dec::MAX, 2),
                ]
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, 2, Dec::ZERO),
                    (dec("10"), 2, dec("100")),
                    (dec("2.5"), 2, dec("6.25")),
                    (dec("123.123"), 3, dec("1866455.185461867")),
                    (dec("-10"), 2, dec("100")),
                    (dec("-10"), 3, dec("-1000")),
                ],
                failing: [
                    (Dec::MAX, 2),
                    (Dec::MIN, 2),
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, 2, Dec::ZERO),
                    (dec("10"), 2, dec("100")),
                    (dec("2.5"), 2, dec("6.25")),
                    (dec("123.123"), 3, dec("1866455.185461867")),
                    (dec("-10"), 2, dec("100")),
                    (dec("-10"), 3, dec("-1000")),
                ],
                failing: [
                    (Dec::MAX, 2),
                    (Dec::MIN, 2),
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, expected);
                assert_eq!(left.checked_pow(right).unwrap(), expected);
            }

            for (left, right) in failing {
                dts!(_0d, left);
                assert!(matches!(left.checked_pow(right), Err(MathError::OverflowPow { .. })));
            }
        }
    );

    dec_test!( checked_sqrt
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (dec("100"), dec("10")),
                    (dec("2"), dec("1.414213562373095048")),
                    (dec("4.84"), dec("2.2")),
                ],
                failing: []
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (dec("100"), dec("10")),
                    (dec("2"), dec("1.414213562373095048")),
                    (dec("4.84"), dec("2.2")),
                ],
                failing: []
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (dec("100"), dec("10")),
                    (dec("2"), dec("1.414213562373095048")),
                    (dec("4.84"), dec("2.2")),
                ],
                failing: [
                    -Dec::ONE
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (dec("100"), dec("10")),
                    (dec("2"), dec("1.414213562373095048")),
                    (dec("4.84"), dec("2.2")),
                ],
                failing: [
                    -Dec::ONE
                ]
            }
        }
        method = |_0d: Dec<_>, passing, failing| {
            for (base, expected) in passing {
                dts!(_0d, base, expected);
                assert_eq!(base.checked_sqrt().unwrap(), expected);
            }

            for base in failing {
                dts!(_0d, base);
                assert!(matches!(base.checked_sqrt(), Err(MathError::NegativeSqrt { .. })));
            }
        }
    );

    dec_test!( checked_rem
        inputs = {
            udec128 = {
                passing: [
                    (dec("10"), dec("4"), dec("2")),
                    (dec("10"), dec("3"), dec("1")),
                    (dec("2.5"), dec("2"), dec("0.5")),
                    (dec("0.3"), dec("2.5"), dec("0.3")),
                    (dec("28"), dec("2.5"), dec("0.5")),
                ]
            }
            udec256 = {
                passing: [
                    (dec("10"), dec("4"), dec("2")),
                    (dec("10"), dec("3"), dec("1")),
                    (dec("2.5"), dec("2"), dec("0.5")),
                    (dec("0.3"), dec("2.5"), dec("0.3")),
                    (dec("28"), dec("2.5"), dec("0.5")),
                ]
            }
            dec128 = {
                passing: [
                    (dec("10"), dec("4"), dec("2")),
                    (dec("10"), dec("3"), dec("1")),
                    (dec("2.5"), dec("2"), dec("0.5")),
                    (dec("0.3"), dec("2.5"), dec("0.3")),
                    (dec("28"), dec("2.5"), dec("0.5")),

                    (dec("-10"), dec("4"), dec("-2")),
                    (dec("28"), dec("-2.5"), dec("0.5")),
                    (dec("-0.3"), dec("-2.5"), dec("-0.3")),
                ]
            }
            dec256 = {
                passing: [
                    (dec("10"), dec("4"), dec("2")),
                    (dec("10"), dec("3"), dec("1")),
                    (dec("2.5"), dec("2"), dec("0.5")),
                    (dec("0.3"), dec("2.5"), dec("0.3")),
                    (dec("28"), dec("2.5"), dec("0.5")),

                    (dec("-10"), dec("4"), dec("-2")),
                    (dec("28"), dec("-2.5"), dec("0.5")),
                    (dec("-0.3"), dec("-2.5"), dec("-0.3")),
                ]
            }
        }
        method = |_0d: Dec<_>, passing| {
            for (base, div, expected) in passing {
                dts!(_0d, base, div, expected);
                assert_eq!(base.checked_rem(div).unwrap(), expected);
            }

            // Division by zero
            assert!(matches!(Dec::TEN.checked_rem(_0d), Err(MathError::DivisionByZero { .. })));
        }
    );

    dec_test!( rem_panic
        attrs = #[should_panic(expected = "division by zero")]
        method = |_0d| {
            let max = bt(_0d, Dec::MAX);
            let _ = max % _0d;
        }
    );

    dec_test!( rem_assign
        attrs = #[allow(clippy::op_ref)]
        method = |_0d| {
            let mut a = bt(_0d, dec("14"));
            a %= bt(_0d, dec("3.3"));
            assert_eq!(a, dec("0.8"));
        }
    );

    dec_test!( saturating_add
        inputs = {
            udec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("11.5")),
                    (Dec::MAX - Dec::TICK, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                ]
            }
            udec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("11.5")),
                    (Dec::MAX - Dec::TICK, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::ONE, Dec::MAX),
                ]
            }
            dec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("11.5")),
                    (Dec::MAX - Dec::TICK, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::ONE, Dec::MAX),

                    (Dec::MIN + Dec::TICK, -Dec::TICK, Dec::MIN),
                    (Dec::MIN, -Dec::TICK, Dec::MIN),
                ]
            }
            dec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("11.5")),
                    (Dec::MAX - Dec::TICK, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::TICK, Dec::MAX),
                    (Dec::MAX, Dec::ONE, Dec::MAX),

                    (Dec::MIN + Dec::TICK, -Dec::TICK, Dec::MIN),
                    (Dec::MIN, -Dec::TICK, Dec::MIN),
                ]
            }
        }
        method = |_0d: Dec<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_add(right), expected);
            }
        }
    );

    dec_test!( saturating_sub
        inputs = {
            udec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("8.5")),
                    (Dec::ZERO + Dec::TICK, Dec::TICK, Dec::ZERO),
                    (Dec::ZERO, Dec::TICK, Dec::ZERO),
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                ]
            }
            udec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("8.5")),
                    (Dec::ZERO + Dec::TICK, Dec::TICK, Dec::ZERO),
                    (Dec::ZERO, Dec::TICK, Dec::ZERO),
                    (Dec::ZERO, Dec::ONE, Dec::ZERO),
                ]
            }
            dec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("8.5")),
                    (Dec::MIN + Dec::TICK, Dec::TICK, Dec::MIN),
                    (Dec::MIN, Dec::TICK, Dec::MIN),
                    (Dec::MIN, Dec::ONE, Dec::MIN),

                    (Dec::MAX - Dec::TICK, -Dec::TICK, Dec::MAX),
                    (Dec::MAX, -Dec::TICK, Dec::MAX),
                    (Dec::MAX, -Dec::ONE, Dec::MAX),
                ]
            }
            dec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("8.5")),
                    (Dec::MIN + Dec::TICK, Dec::TICK, Dec::MIN),
                    (Dec::MIN, Dec::TICK, Dec::MIN),
                    (Dec::MIN, Dec::ONE, Dec::MIN),

                    (Dec::MAX - Dec::TICK, -Dec::TICK, Dec::MAX),
                    (Dec::MAX, -Dec::TICK, Dec::MAX),
                    (Dec::MAX, -Dec::ONE, Dec::MAX),
                ]
            }
        }
        method = |_0d: Dec<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_sub(right), expected);
            }
        }
    );

    dec_test!( saturating_mul
        inputs = {
            udec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("15")),
                    (Dec::MAX / dec("2"), dec("2"), Dec::MAX - Dec::TICK),
                    (Dec::MAX / dec("2") + Dec::TICK, dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2") + Dec::ONE, dec("2"), Dec::MAX),
                ]
            }
            udec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("15")),
                    (Dec::MAX / dec("2"), dec("2"), Dec::MAX - Dec::TICK),
                    (Dec::MAX / dec("2") + Dec::TICK, dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2") + Dec::ONE, dec("2"), Dec::MAX),
                ]
            }
            dec128 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("15")),
                    (dec("10"), dec("-1.5"), dec("-15")),
                    (Dec::MAX / dec("2"), dec("2"), Dec::MAX - Dec::TICK),
                    (Dec::MAX / dec("2") + Dec::TICK, dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2") + Dec::ONE, dec("2"), Dec::MAX),

                    (Dec::MIN / dec("2"), dec("2"), Dec::MIN),
                    (Dec::MIN / dec("2") - Dec::TICK, dec("2"), Dec::MIN),

                    (Dec::MIN / dec("2"), - dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2"), - dec("2"), Dec::MIN + Dec::TICK * dec("2")),
                ]
            }
            dec256 = {
                passing: [
                    (dec("10"), dec("1.5"), dec("15")),
                    (dec("10"), dec("-1.5"), dec("-15")),
                    (Dec::MAX / dec("2"), dec("2"), Dec::MAX - Dec::TICK),
                    (Dec::MAX / dec("2") + Dec::TICK, dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2") + Dec::ONE, dec("2"), Dec::MAX),

                    (Dec::MIN / dec("2"), dec("2"), Dec::MIN),
                    (Dec::MIN / dec("2") - Dec::TICK, dec("2"), Dec::MIN),

                    (Dec::MIN / dec("2"), - dec("2"), Dec::MAX),
                    (Dec::MAX / dec("2"), - dec("2"), Dec::MIN + Dec::TICK * dec("2")),
                ]
            }
        }
        method = |_0d: Dec<_>, passing| {
            for (left, right, expected) in passing {
                dts!(_0d, left, right, expected);
                assert_eq!(left.saturating_mul(right), expected);
            }
        }
    );

    dec_test!( saturating_pow
        inputs = {
            udec128 = {
                passing: [
                    (dec("10"), 2, dec("100")),
                    (Dec::MAX / dec("2"), 2, Dec::MAX),
                    (Dec::MAX, 2, Dec::MAX),
                    (Dec::MAX, 3, Dec::MAX),

                ]
            }
            udec256 = {
                passing: [
                    (dec("10"), 2, dec("100")),
                    (Dec::MAX / dec("2"), 2, Dec::MAX),
                    (Dec::MAX, 2, Dec::MAX),
                    (Dec::MAX, 3, Dec::MAX),
                ]
            }
            dec128 = {
                passing: [
                    (dec("10"), 2, dec("100")),
                    (Dec::MAX / dec("2"), 2, Dec::MAX),
                    (Dec::MAX, 2, Dec::MAX),
                    (Dec::MAX, 3, Dec::MAX),

                    (dec("-10"), 2, dec("100")),
                    (dec("-10"), 3, dec("-1000")),
                    (Dec::MIN, 2, Dec::MAX),
                    (Dec::MIN, 3, Dec::MIN),
                ]
            }
            dec256 = {
                passing: [
                    (dec("10"), 2, dec("100")),
                    (Dec::MAX / dec("2"), 2, Dec::MAX),
                    (Dec::MAX, 2, Dec::MAX),
                    (Dec::MAX, 3, Dec::MAX),

                    (dec("-10"), 2, dec("100")),
                    (dec("-10"), 3, dec("-1000")),
                    (Dec::MIN, 2, Dec::MAX),
                    (Dec::MIN, 3, Dec::MIN),
                ]
            }
        }
        method = |_0d: Dec<_>, passing| {
            for (base, exp, expected) in passing {
                dts!(_0d, base, expected);
                assert_eq!(base.saturating_pow(exp), expected);
            }
        }
    );
}
