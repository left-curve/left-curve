use {
    crate::{
        Dec, FixedPoint, Int, Integer, IsZero, MathError, MathResult, NextNumber, NumberConst, Sign,
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
    <Int<U> as NextNumber>::Next: Number + IsZero + Copy + ToString,
{
    fn checked_add(self, other: Self) -> MathResult<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> MathResult<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn checked_mul(self, other: Self) -> MathResult<Self> {
        let next_result = self
            .0
            .checked_full_mul(*other.numerator())?
            .checked_div(Self::DECIMAL_FRACTION.into())?;

        next_result
            .try_into()
            .map(Self)
            .map_err(|_| MathError::overflow_conversion::<_, Int<U>>(next_result))
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
mod tests {

    use {
        crate::{dts, int_test, test_utils::bt, Int, MathError, Number, NumberConst},
        bnum::types::{I256, U256},
    };

    int_test!( add,
        Specific
        u128 = [[ // Passing cases
                    (0_u128, 0_u128, 0_u128),
                    (0, u128::MAX, u128::MAX),
                    (10, 20, 30),
                ],
                [ // Failing cases
                    (u128::MAX, 1_u128)
                ]]

        u256 = [[ // Passing cases
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::ZERO, U256::MAX, U256::MAX),
                    (U256::from(10_u32), U256::from(20_u32), U256::from(30_u32)),
                ],
                [  // Failing cases
                    (U256::MAX, U256::ONE)
                ]]

        i128 = [[ // Passing cases
                    (0_i128, 0_i128, 0_i128),
                    (0, i128::MAX, i128::MAX),
                    (0, i128::MIN, i128::MIN),
                    (10, 20, 30),
                    (-10, 20, 10),
                    (10, -20, -10),
                    (-10, -20, -30),
                ],
                [ // Failing cases
                    (i128::MAX, 1),
                    (i128::MIN, -1),
                ]]

        i256 = [[ // Passing cases
                    (I256::ZERO, I256::ZERO, I256::ZERO),
                    (I256::ZERO, I256::MAX, I256::MAX),
                    (I256::ZERO, I256::MIN, I256::MIN),
                    (I256::from(10), I256::from(20), I256::from(30)),
                    (I256::from(-10), I256::from(20), I256::from(10)),
                    (I256::from(10), I256::from(-20), I256::from(-10)),
                    (I256::from(-10), I256::from(-20), I256::from(-30)),
                ],
                [ // Failing cases
                    (I256::MAX, I256::ONE),
                    (I256::MIN, -I256::ONE),
                ]]
        => |_0, samples, failing_samples| {
            for (left, right, expected) in samples {
                let left = Int::from(left);
                let right = Int::from(right);
                let expected = Int::from(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left + right, expected);
            }

            for (left, right) in failing_samples {
                let left = Int::from(left);
                let right = Int::from(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_add(right), Err(MathError::OverflowAdd { .. })));
            }
        }
    );

    int_test!( add_panic,
        NoArgs
        attrs = #[should_panic(expected = "addition overflow")]
        => |_0| {
            let max = bt(_0, Int::MAX);
            let one = bt(_0,Int::ONE);
            let _ = max + one;
        }
    );

    int_test!( add_assign,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u64));
            a += bt(_0, Int::from(2_u64));
            assert_eq!(a, bt(_0, Int::from(16_u64)));
        }
    );

    int_test!( sub,
        Specific
        u128 = [[ // Passing cases
                    (0_u128, 0_u128, 0_u128),
                    (u128::MAX, u128::MAX, 0),
                    (30, 20, 10),
                ],
                [ // Failing cases
                    (1_u128, 2_u128)
                ]]

        u256 = [[ // Passing cases
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::MAX, U256::MAX, U256::ZERO),
                    (U256::from(30_u32), U256::from(10_u32), U256::from(20_u32)),
                ],
                [  // Failing cases
                    (U256::ONE, U256::from(2_u32))
                ]]

        i128 = [[ // Passing cases
                    (0_i128, 0_i128, 0_i128),
                    (i128::MAX, i128::MAX, 0),
                    (i128::MIN, i128::MIN, 0),
                    (0, i128::MIN + i128::ONE, i128::MAX),
                    (30, 20, 10),
                    (-10, 20, -30),
                    (10, -20, 30),
                    (-10, -20, 10),
                ],
                [ // Failing cases
                    (i128::MIN, 1),
                    (i128::MAX, -1),
                ]]

        i256 = [[ // Passing cases
                    (I256::ZERO, I256::ZERO, I256::ZERO),
                    (I256::MAX, I256::MAX, I256::ZERO),
                    (I256::MIN, I256::MIN, I256::ZERO),
                    (I256::ZERO, I256::MIN + I256::ONE, I256::MAX),
                    (I256::from(30), I256::from(20), I256::from(10)),
                    (I256::from(-10), I256::from(20), I256::from(-30)),
                    (I256::from(10), I256::from(-20), I256::from(30)),
                    (I256::from(-10), I256::from(-20), I256::from(10)),
                ],
                [ // Failing cases
                    (I256::MIN, I256::ONE),
                    (I256::MAX, -I256::ONE),
                ]]
        => |_0, samples, failing_samples| {
            for (left, right, expected) in samples {
                let left = Int::from(left);
                let right = Int::from(right);
                let expected = Int::from(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left - right, expected);
            }

            for (left, right) in failing_samples {
                let left = Int::from(left);
                let right = Int::from(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_sub(right), Err(MathError::OverflowSub { .. })));
            }
        }
    );

    int_test!( sub_panic,
        NoArgs
        attrs = #[should_panic(expected = "subtraction overflow")]
        => |_0| {
            let max = bt(_0, Int::MIN);
            let one = bt(_0, Int::ONE);
            let _ = max - one;
        }
    );

    int_test!( sub_assign,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u64));
            a -= bt(_0, Int::from(2_u64));
            assert_eq!(a, bt(_0, Int::from(12_u64)));
        }
    );

    int_test!( mul,
        Specific
        u128 = [[ // Passing cases
                    (0_u128, 0_u128, 0_u128),
                    (u128::MAX, 0, 0),
                    (30, 20, 600),
                ],
                [ // Failing cases
                    (u128::MAX, 2_u128)
                ]]

        u256 = [[ // Passing cases
                    (U256::ZERO, U256::ZERO, U256::ZERO),
                    (U256::MAX, U256::ZERO, U256::ZERO),
                    (U256::from(30_u32), U256::from(10_u32), U256::from(300_u32)),
                ],
                [  // Failing cases
                    (U256::MAX, U256::from(2_u32))
                ]]

        i128 = [[ // Passing cases
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
                [ // Failing cases
                    (i128::MIN, 2),
                    (i128::MIN, -2),
                    (i128::MAX, 2),
                    (i128::MAX, -2),
                ]]

        i256 = [[ // Passing cases
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
                [ // Failing cases
                    (I256::MIN, I256::from(2)),
                    (I256::MIN, I256::from(-2)),
                    (I256::MAX, I256::from(2)),
                    (I256::MAX, I256::from(-2)),
                ]]
        => |_0, samples, failing_samples| {
            for (left, right, expected) in samples {
                let left = Int::from(left);
                let right = Int::from(right);
                let expected = Int::from(expected);
                dts!(_0, left, right, expected);
                assert_eq!(left * right, expected);
            }

            for (left, right) in failing_samples {
                let left = Int::from(left);
                let right = Int::from(right);
                dts!(_0, left, right);
                assert!(matches!(left.checked_mul(right), Err(MathError::OverflowMul { .. })));
            }
        }
    );

    int_test!( mul_panic,
        NoArgs
        attrs = #[should_panic(expected = "multiplication overflow")]
        => |_0| {
            let max = bt(_0, Int::MAX);
            let one = bt(_0, Int::from(2_u64));
            let _ = max * one;
        }
    );

    int_test!( mul_assign,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u64));
            a *= bt(_0, Int::from(2_u64));
            assert_eq!(a, bt(_0, Int::from(28_u64)));
        }
    );

    int_test!( div,
        Specific
        u128 = [[ // Passing cases
                    (u128::MAX, 1_u128, u128::MAX),
                    (0, 1, 0),
                    (300, 20, 15),
                    (30, 20, 1),
                ]]

        u256 = [[ // Passing cases
                    (U256::MAX, U256::ONE, U256::MAX),
                    (U256::ZERO, U256::ONE, U256::ZERO),
                    (U256::from(300_u32), U256::from(20_u32), U256::from(15_u32)),
                    (U256::from(30_u32), U256::from(20_u32), U256::from(1_u32)),
                ]]

        i128 = [[ // Passing cases
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
                ]]

        i256 = [[ // Passing cases
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
                ]]
        => |_0, samples| {
            for (left, right, expected) in samples {
                let left = Int::from(left);
                let right = Int::from(right);
                let expected = Int::from(expected);
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

    int_test!( div_panic,
        NoArgs
        attrs = #[should_panic(expected = "division by zero")]
        => |_0| {
            let max = bt(_0, Int::MAX);
            let _ = max / _0;
        }
    );

    int_test!( div_assign,
        NoArgs
        attrs = #[allow(clippy::op_ref)]
        => |_0| {
            let mut a = bt(_0, Int::from(14_u64));
            a /= bt(_0, Int::from(2_u64));
            assert_eq!(a, bt(_0, Int::from(7_u64)));
        }
    );
}
