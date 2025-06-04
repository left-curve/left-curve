use {
    crate::{Dec, Int, MathError, MathResult, NumberConst},
    bnum::types::{I256, I512, U256, U512},
    std::any::type_name,
};

/// Describes a number that can take on negative values.
pub trait Sign: Sized + Copy {
    /// Return the number's absolute value.
    ///
    /// ## Note
    ///
    /// This method is fallible, because taking the absolute value of a
    /// [two's complement](https://en.wikipedia.org/wiki/Two%27s_complement)
    /// number's minimum value (i.e. the maximally negative value) leads to
    /// overflow.
    fn checked_abs(self) -> MathResult<Self>;

    #[inline]
    fn checked_abs_assign(&mut self) -> MathResult<()> {
        *self = self.checked_abs()?;
        Ok(())
    }

    /// Return true if the number is negative; false if it's zero or positive.
    fn is_negative(&self) -> bool;

    /// Return true if the number is positive; false if it's zero or negative.
    fn is_positive(&self) -> bool;

    /// Return true if the number is zero or positive; false if it's negative.
    #[inline]
    fn is_non_negative(&self) -> bool {
        !self.is_negative()
    }

    /// Return true if the number is zero or negative; false if it's positive.
    #[inline]
    fn is_non_positive(&self) -> bool {
        !self.is_positive()
    }

    /// Return the number's negative value.
    fn checked_neg(self) -> MathResult<Self>;
}

// ------------------------------------ int ------------------------------------

impl<U> Sign for Int<U>
where
    U: Sign,
{
    fn checked_abs(self) -> MathResult<Self> {
        self.0.checked_abs().map(Self)
    }

    fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    fn is_positive(&self) -> bool {
        self.0.is_positive()
    }

    fn checked_neg(self) -> MathResult<Self> {
        self.0.checked_neg().map(Self)
    }
}

// ------------------------------------ dec ------------------------------------

impl<U, const S: u32> Sign for Dec<U, S>
where
    U: Sign,
{
    fn checked_abs(self) -> MathResult<Self> {
        self.0.checked_abs().map(Self)
    }

    fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    fn is_positive(&self) -> bool {
        self.0.is_positive()
    }

    fn checked_neg(self) -> MathResult<Self> {
        self.0.checked_neg().map(Self)
    }
}

// --------------------------------- unsigned ----------------------------------

macro_rules! impl_sign_unsigned {
    ($t:ty) => {
        impl Sign for $t {
            fn checked_abs(self) -> MathResult<Self> {
                Ok(self)
            }

            fn is_negative(&self) -> bool {
                false
            }

            fn is_positive(&self) -> bool {
                *self > Self::ZERO
            }

            fn checked_neg(self) -> MathResult<Self> {
                self.checked_neg().ok_or(MathError::InvalidNegation)
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_sign_unsigned!($t);
        )+
    };
}

impl_sign_unsigned!(u8, u16, u32, u64, u128, U256, U512);

// ---------------------------------- signed -----------------------------------

macro_rules! impl_sign_signed {
    ($t:ty) => {
        impl Sign for $t {
            fn checked_abs(self) -> MathResult<Self> {
                if self == Self::MIN {
                    Err(MathError::overflow_conversion::<_, Self>(self))
                } else {
                    Ok(self.abs())
                }
            }

            fn is_negative(&self) -> bool {
                *self < Self::ZERO
            }

            fn is_positive(&self) -> bool {
                *self > Self::ZERO
            }

            fn checked_neg(self) -> MathResult<Self> {
                self.checked_neg().ok_or(MathError::OverflowNegation {
                    ty: type_name::<Self>(),
                    value: self.to_string(),
                })
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_sign_signed!($t);
        )+
    };
}

impl_sign_signed!(i8, i16, i32, i64, i128, I256, I512);

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{Int, MathError, NumberConst, Sign, int_test, test_utils::bt},
        bnum::types::{I256, U256},
    };

    int_test!( sign
        inputs = {
            u128 = {
                passing: [
                    (u128::ZERO, false, u128::ZERO),
                    (u128::MAX, false, u128::MAX),
                ],
                failing: []
            }
            u256 = {
                passing: [
                    (U256::ZERO, false, U256::ZERO),
                    (U256::MAX, false, U256::MAX),
                ],
                failing: []
            }
            i128 = {
                passing: [
                    (i128::ZERO, false, i128::ZERO),
                    (i128::MAX, false, i128::MAX),
                    (-i128::ONE, true, i128::ONE),
                    (-i128::MAX, true, i128::MAX),
                ],
                failing: [
                    i128::MIN,
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, false, I256::ZERO),
                    (I256::MAX, false, I256::MAX),
                    (-I256::ONE, true, I256::ONE),
                    (-I256::MAX, true, I256::MAX),
                ],
                failing: [
                    I256::MIN,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (base, sign, abs) in passing {
                let base = bt(_0, Int::new(base));
                assert_eq!(base.is_negative(), sign);
                assert_eq!(base.checked_abs().unwrap(), Int::new(abs));
            }

            for failing in failing {
                let base = bt(_0, Int::new(failing));
                assert!(matches!(base.checked_abs(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );

    int_test!( checked_neg_unsigned
        inputs = {
            u128 = {
                passing: [
                    (u128::ZERO, u128::ZERO),
                    (u128::MIN, u128::MIN),
                ],
                failing: [
                    u128::MAX,
                    u128::ONE,
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, U256::ZERO),
                    (U256::MIN, U256::MIN),
                ],
                failing: [
                    U256::MAX,
                    U256::ONE,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (base, neg) in passing {
                let base = bt(_0, Int::new(base));
                println!("base: {}", base);
                println!("base.checked_neg(): {:?}", base.checked_neg());
                assert_eq!(base.checked_neg().unwrap(), Int::new(neg));
            }

            for failing in failing {
                let base = bt(_0, Int::new(failing));
                println!("failing: {}", base);
                println!("failing.checked_neg(): {:?}", base.checked_neg());
                assert!(matches!(base.checked_neg(), Err(MathError::InvalidNegation { .. })));
            }
        }
    );

    int_test!( checked_neg_signed
        inputs = {
            i128 = {
                passing: [
                    (i128::ZERO, i128::ZERO),
                    (i128::MAX, -i128::MAX),
                    (i128::ONE, -i128::ONE),
                    (-i128::ONE, i128::ONE),
                    (-i128::MAX, i128::MAX),
                ],
                failing: [
                    i128::MIN,
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, I256::ZERO),
                    (I256::MAX, -I256::MAX),
                    (I256::ONE, -I256::ONE),
                    (-I256::ONE, I256::ONE),
                    (-I256::MAX, I256::MAX),
                ],
                failing: [
                    I256::MIN,
                ]
            }
        }
        method = |_0, passing, failing| {
            for (base, neg) in passing {
                let base = bt(_0, Int::new(base));
                assert_eq!(base.checked_neg().unwrap(), Int::new(neg));
            }

            for failing in failing {
                let base = bt(_0, Int::new(failing));
                assert!(matches!(base.checked_neg(), Err(MathError::OverflowNegation { .. })));
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use crate::{Dec, FixedPoint, MathError, NumberConst, Sign, dec_test, test_utils::dt};

    dec_test!( sign
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, false, Dec::ZERO),
                    (Dec::MAX, false, Dec::MAX),
                ],
                failing: []
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, false, Dec::ZERO),
                    (Dec::MAX, false, Dec::MAX),
                ],
                failing: []
            }
            dec128 = {
                passing: [
                    (Dec::ZERO, false, Dec::ZERO),
                    (Dec::MAX, false, Dec::MAX),
                    (-Dec::ONE, true, Dec::ONE),
                    (Dec::MIN + Dec::TICK, true, Dec::MAX),
                ],
                failing: [
                    Dec::MIN,
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, false, Dec::ZERO),
                    (Dec::MAX, false, Dec::MAX),
                    (-Dec::ONE, true, Dec::ONE),
                    (Dec::MIN + Dec::TICK, true, Dec::MAX),
                ],
                failing: [
                    Dec::MIN,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (base, sign, abs) in passing {
                dt(_0d, base);
                assert_eq!(base.is_negative(), sign);
                assert_eq!(base.checked_abs().unwrap(), abs);
            }

            for failing in failing {
                dt(_0d, failing);
                assert!(matches!(failing.checked_abs(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );

    dec_test!( checked_neg_unsigned
        inputs = {
            udec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (Dec::MIN, Dec::MIN),
                ],
                failing: [
                    Dec::MAX,
                    Dec::ONE,
                ]
            }
            udec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (Dec::MIN, Dec::MIN),
                ],
                failing: [
                    Dec::MAX,
                    Dec::ONE,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (base, neg) in passing {
                dt(_0d, base);
                assert_eq!(base.checked_neg().unwrap(), neg);
            }

            for failing in failing {
                dt(_0d, failing);
                assert!(matches!(failing.checked_neg(), Err(MathError::InvalidNegation { .. })));
            }
        }
    );

    dec_test!( checked_neg_signed
        inputs = {
            dec128 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, -Dec::MAX),
                    (Dec::ONE, -Dec::ONE),
                    (-Dec::ONE, Dec::ONE),
                    (-Dec::MAX, Dec::MAX),
                ],
                failing: [
                    Dec::MIN,
                ]
            }
            dec256 = {
                passing: [
                    (Dec::ZERO, Dec::ZERO),
                    (Dec::MAX, -Dec::MAX),
                    (Dec::ONE, -Dec::ONE),
                    (-Dec::ONE, Dec::ONE),
                    (-Dec::MAX, Dec::MAX),
                ],
                failing: [
                    Dec::MIN,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (base, neg) in passing {
                dt(_0d, base);
                assert_eq!(base.checked_neg().unwrap(), neg);
            }

            for failing in failing {
                dt(_0d, failing);
                assert!(matches!(failing.checked_neg(), Err(MathError::OverflowNegation { .. })));
            }
        }
    );
}
