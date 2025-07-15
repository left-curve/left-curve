use {
    crate::{Dec, Int, Integer, IsZero, MathError, MathResult, Number, NumberConst, Sign},
    bnum::types::{I256, I512, U256, U512},
    std::fmt::Display,
};

pub trait UnaryNumber: Sized + Copy {
    fn checked_pow(self, exponent: u32) -> MathResult<Self>;

    fn checked_sqrt(self) -> MathResult<Self>;

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

    fn saturating_pow(self, exp: u32) -> Self;

    #[inline]
    fn saturating_pow_assign(&mut self, exp: u32) {
        *self = self.saturating_pow(exp);
    }
}

// ------------------------------------ int ------------------------------------

impl<U> UnaryNumber for Int<U>
where
    U: UnaryNumber,
{
    fn checked_pow(self, exp: u32) -> MathResult<Self> {
        self.0.checked_pow(exp).map(Self)
    }

    fn checked_sqrt(self) -> MathResult<Self> {
        self.0.checked_sqrt().map(Self)
    }

    fn saturating_pow(self, exp: u32) -> Self {
        Self(self.0.saturating_pow(exp))
    }
}

// ------------------------------------ dec ------------------------------------

impl<U, const S: u32> UnaryNumber for Dec<U, S>
where
    U: Number + UnaryNumber + NumberConst + PartialOrd,
    Self: Copy + Number + NumberConst + Display + Sign,
{
    fn checked_pow(mut self, mut exp: u32) -> MathResult<Self> {
        (|| {
            if exp == 0 {
                return Ok(Self::ONE);
            }

            let mut y = Self::ONE;

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

macro_rules! impl_unary_number {
    ($t:ty) => {
        impl UnaryNumber for $t {
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

            fn saturating_pow(self, other: u32) -> Self {
                self.saturating_pow(other)
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_unary_number!($t);
        )+
    };
}

impl_unary_number! {
    u8, u16, u32, u64, u128, U256, U512,
    i8, i16, i32, i64, i128, I256, I512,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{Int, MathError, UnaryNumber, dts, int_test, test_utils::bt},
        bnum::types::{I256, U256},
    };

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
}

#[cfg(test)]
mod dec_tests {
    use crate::{Dec, MathError, NumberConst, UnaryNumber, dec_test, dts, test_utils::dec};

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
        method = |_0d: Dec<_, 18>, passing, failing| {
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
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (base, expected) in passing {
                dts!(_0d, base, expected);
                // assert_eq!(base.checked_sqrt().unwrap(), expected);
            }

            for base in failing {
                dts!(_0d, base);
                // assert!(matches!(base.checked_sqrt(), Err(MathError::NegativeSqrt { .. })));
            }
        }
    );
}
