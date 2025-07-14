use crate::{
    Dec, FixedPoint, Fraction, Int, IsZero, MathError, MathResult, MultiplyRatio, Number,
    NumberConst,
};

/// Describes operations between a number and a decimal type.
pub trait MultiplyFraction<F, U>: Sized + Copy
where
    F: Fraction<U>,
{
    fn checked_mul_dec(self, rhs: F) -> MathResult<Self>;

    fn checked_mul_dec_floor(self, rhs: F) -> MathResult<Self>;

    fn checked_mul_dec_ceil(self, rhs: F) -> MathResult<Self>;

    fn checked_div_dec(self, rhs: F) -> MathResult<Self>;

    fn checked_div_dec_floor(self, rhs: F) -> MathResult<Self>;

    fn checked_div_dec_ceil(self, rhs: F) -> MathResult<Self>;

    #[inline]
    fn checked_mul_dec_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_mul_dec(rhs)?;
        Ok(())
    }

    #[inline]
    fn checked_mul_dec_floor_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_mul_dec_floor(rhs)?;
        Ok(())
    }

    #[inline]
    fn checked_mul_dec_ceil_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_mul_dec_ceil(rhs)?;
        Ok(())
    }

    #[inline]
    fn checked_div_dec_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_div_dec(rhs)?;
        Ok(())
    }

    #[inline]
    fn checked_div_dec_floor_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_div_dec_floor(rhs)?;
        Ok(())
    }

    #[inline]
    fn checked_div_dec_ceil_assign(&mut self, rhs: F) -> MathResult<()> {
        *self = self.checked_div_dec_ceil(rhs)?;
        Ok(())
    }
}

// ------------------------------------ int ------------------------------------

impl<U, const S: u32> MultiplyFraction<Dec<U, S>, U> for Int<U>
where
    Int<U>: IsZero + NumberConst + MultiplyRatio + ToString + Copy,
    Dec<U, S>: IsZero + Fraction<U>,
{
    fn checked_mul_dec(self, rhs: Dec<U, S>) -> MathResult<Self> {
        // If either left or right hand side is zero, then simply return zero.
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio(rhs.numerator(), Dec::<U, S>::denominator())
    }

    fn checked_mul_dec_floor(self, rhs: Dec<U, S>) -> MathResult<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(rhs.numerator(), Dec::<U, S>::denominator())
    }

    fn checked_mul_dec_ceil(self, rhs: Dec<U, S>) -> MathResult<Self> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(rhs.numerator(), Dec::<U, S>::denominator())
    }

    fn checked_div_dec(self, rhs: Dec<U, S>) -> MathResult<Self> {
        // If right hand side is zero, throw error, because you can't divide any
        // number by zero.
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        // If left hand side is zero, and we know right hand size is positive,
        // then simply return zero.
        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio(Dec::<U, S>::denominator(), rhs.numerator())
    }

    fn checked_div_dec_floor(self, rhs: Dec<U, S>) -> MathResult<Self> {
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_floor(Dec::<U, S>::denominator(), rhs.numerator())
    }

    fn checked_div_dec_ceil(self, rhs: Dec<U, S>) -> MathResult<Self> {
        if rhs.is_zero() {
            return Err(MathError::division_by_zero(self));
        }

        if self.is_zero() {
            return Ok(Self::ZERO);
        }

        self.checked_multiply_ratio_ceil(Dec::<U, S>::denominator(), rhs.numerator())
    }
}

// ------------------------------------ dec ------------------------------------

impl<U, const S: u32, const S1: u32> MultiplyFraction<Dec<U, S1>, U> for Dec<U, S>
where
    Dec<U, S>: Fraction<U> + Copy + Number<Dec<U, S1>> + FixedPoint<U>,
    Dec<U, S1>: Fraction<U> + Copy,
    Int<U>: MultiplyFraction<Dec<U, S1>, U> + MultiplyRatio,
{
    fn checked_mul_dec(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.checked_mul(rhs)
    }

    fn checked_mul_dec_floor(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.0.checked_mul_dec_floor(rhs).map(Self)
    }

    fn checked_mul_dec_ceil(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.0.checked_mul_dec_ceil(rhs).map(Self)
    }

    fn checked_div_dec(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.checked_div(rhs)
    }

    fn checked_div_dec_floor(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.numerator()
            .checked_multiply_ratio_floor(Dec::<U, S1>::denominator(), rhs.numerator())
            .map(Self)
    }

    fn checked_div_dec_ceil(self, rhs: Dec<U, S1>) -> MathResult<Self> {
        self.numerator()
            .checked_multiply_ratio_ceil(Dec::<U, S1>::denominator(), rhs.numerator())
            .map(Self)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{
            Dec, Dec128, Dec256, Int, MathError, MultiplyFraction, NumberConst, Udec128, Udec256,
            int_test, test_utils::bt,
        },
        bnum::types::{I256, U256},
    };

    int_test!( checked_mul_dec
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new(2), 20_u128),
                    (10_u128, Udec128::new_percent(150), 15_u128),
                    (10_u128, Udec128::new_percent(50), 5_u128),
                    (11_u128, Udec128::new_percent(50), 5_u128),
                    (9_u128, Udec128::new_percent(50), 4_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new(2), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new_percent(150), U256::from(15_u128)),
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(5_u128)),
                    (U256::from(11_u128), Udec256::new_percent(50), U256::from(5_u128)),
                    (U256::from(9_u128), Udec256::new_percent(50), U256::from(4_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new(2), 20_i128),
                    (10_i128, Dec128::new_percent(150), 15_i128),
                    (10_i128, Dec128::new_percent(50), 5_i128),
                    (11_i128, Dec128::new_percent(50), 5_i128),
                    (9_i128, Dec128::new_percent(50), 4_i128),

                    (-10_i128, Dec128::new_percent(50), -5_i128),
                    (-11_i128, Dec128::new_percent(50), -5_i128),
                    (-9_i128, Dec128::new_percent(50), -4_i128),

                    (10_i128, Dec128::new_percent(-50), -5_i128),
                    (11_i128, Dec128::new_percent(-50), -5_i128),
                    (9_i128, Dec128::new_percent(-50), -4_i128),

                    (-10_i128, Dec128::new_percent(-50), 5_i128),
                    (-11_i128, Dec128::new_percent(-50), 5_i128),
                    (-9_i128, Dec128::new_percent(-50), 4_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new(2), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new_percent(150), I256::from(15_i128)),
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(4_i128)),

                    (I256::from(-10_i128), Dec256::new_percent(50), I256::from(-5_i128)),
                    (I256::from(-11_i128), Dec256::new_percent(50), I256::from(-5_i128)),
                    (I256::from(-9_i128), Dec256::new_percent(50), I256::from(-4_i128)),

                    (I256::from(10_i128), Dec256::new_percent(-50), I256::from(-5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(-50), I256::from(-5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(-50), I256::from(-4_i128)),

                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(4_i128)),
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_mul_dec(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }
        }
    );

    int_test!( checked_mul_dec_floor
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new(2), 20_u128),
                    (10_u128, Udec128::new_percent(150), 15_u128),
                    (10_u128, Udec128::new_percent(50), 5_u128),
                    (11_u128, Udec128::new_percent(50), 5_u128),
                    (9_u128, Udec128::new_percent(50), 4_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new(2), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new_percent(150), U256::from(15_u128)),
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(5_u128)),
                    (U256::from(11_u128), Udec256::new_percent(50), U256::from(5_u128)),
                    (U256::from(9_u128), Udec256::new_percent(50), U256::from(4_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new(2), 20_i128),
                    (10_i128, Dec128::new_percent(150), 15_i128),
                    (10_i128, Dec128::new_percent(50), 5_i128),
                    (11_i128, Dec128::new_percent(50), 5_i128),
                    (9_i128, Dec128::new_percent(50), 4_i128),

                    (-10_i128, Dec128::new_percent(50), -5_i128),
                    (-11_i128, Dec128::new_percent(50), -6_i128),
                    (-9_i128, Dec128::new_percent(50), -5_i128),

                    (10_i128, Dec128::new_percent(-50), -5_i128),
                    (11_i128, Dec128::new_percent(-50), -6_i128),
                    (9_i128, Dec128::new_percent(-50), -5_i128),

                    (-10_i128, Dec128::new_percent(-50), 5_i128),
                    (-11_i128, Dec128::new_percent(-50), 5_i128),
                    (-9_i128, Dec128::new_percent(-50), 4_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new(2), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new_percent(150), I256::from(15_i128)),
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(4_i128)),

                    (I256::from(-10_i128), Dec256::new_percent(50), I256::from(-5_i128)),
                    (I256::from(-11_i128), Dec256::new_percent(50), I256::from(-6_i128)),
                    (I256::from(-9_i128), Dec256::new_percent(50), I256::from(-5_i128)),

                    (I256::from(10_i128), Dec256::new_percent(-50), I256::from(-5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(-50), I256::from(-6_i128)),
                    (I256::from(9_i128), Dec256::new_percent(-50), I256::from(-5_i128)),

                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(4_i128))
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_mul_dec_floor(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }
        }
    );

    int_test!( checked_mul_dec_ceil
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new(2), 20_u128),
                    (10_u128, Udec128::new_percent(150), 15_u128),
                    (10_u128, Udec128::new_percent(50), 5_u128),
                    (11_u128, Udec128::new_percent(50), 6_u128),
                    (9_u128, Udec128::new_percent(50), 5_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new(2), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new_percent(150), U256::from(15_u128)),
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(5_u128)),
                    (U256::from(11_u128), Udec256::new_percent(50), U256::from(6_u128)),
                    (U256::from(9_u128), Udec256::new_percent(50), U256::from(5_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new(2), 20_i128),
                    (10_i128, Dec128::new_percent(150), 15_i128),
                    (10_i128, Dec128::new_percent(50), 5_i128),
                    (11_i128, Dec128::new_percent(50), 6_i128),
                    (9_i128, Dec128::new_percent(50), 5_i128),

                    (-10_i128, Dec128::new_percent(50), -5_i128),
                    (-11_i128, Dec128::new_percent(50), -5_i128),
                    (-9_i128, Dec128::new_percent(50), -4_i128),

                    (10_i128, Dec128::new_percent(-50), -5_i128),
                    (11_i128, Dec128::new_percent(-50), -5_i128),
                    (9_i128, Dec128::new_percent(-50), -4_i128),

                    (-10_i128, Dec128::new_percent(-50), 5_i128),
                    (-11_i128, Dec128::new_percent(-50), 6_i128),
                    (-9_i128, Dec128::new_percent(-50), 5_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new(2), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new_percent(150), I256::from(15_i128)),
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(6_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(5_i128)),

                    (I256::from(-10_i128), Dec256::new_percent(50), I256::from(-5_i128)),
                    (I256::from(-11_i128), Dec256::new_percent(50), I256::from(-5_i128)),
                    (I256::from(-9_i128), Dec256::new_percent(50), I256::from(-4_i128)),

                    (I256::from(10_i128), Dec256::new_percent(-50), I256::from(-5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(-50), I256::from(-5_i128)),
                    (I256::from(9_i128), Dec256::new_percent(-50), I256::from(-4_i128)),

                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new_percent(50), I256::from(6_i128)),
                    (I256::from(9_i128), Dec256::new_percent(50), I256::from(5_i128)),
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_mul_dec_ceil(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }
        }
    );

    int_test!( checked_div_dec
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new_percent(50), 20_u128),
                    (10_u128, Udec128::new(2), 5_u128),
                    (9_u128, Udec128::new(2), 4_u128),
                    (11_u128, Udec128::new(2), 5_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new(2), U256::from(5_u128)),
                    (U256::from(9_u128), Udec256::new(2), U256::from(4_u128)),
                    (U256::from(11_u128), Udec256::new(2), U256::from(5_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new_percent(50), 20_i128),
                    (10_i128, Dec128::new(2), 5_i128),
                    (9_i128, Dec128::new(2), 4_i128),
                    (11_i128, Dec128::new(2), 5_i128),

                    (-10_i128, Dec128::new(2), -5_i128),
                    (-9_i128, Dec128::new(2), -4_i128),
                    (-11_i128, Dec128::new(2), -5_i128),

                    (10_i128, Dec128::new(-2), -5_i128),
                    (9_i128, Dec128::new(-2), -4_i128),
                    (11_i128, Dec128::new(-2), -5_i128),

                    (-10_i128, Dec128::new(-2), 5_i128),
                    (-9_i128, Dec128::new(-2), 4_i128),
                    (-11_i128, Dec128::new(-2), 5_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new(2), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new(2), I256::from(4_i128)),
                    (I256::from(11_i128), Dec256::new(2), I256::from(5_i128)),

                    (I256::from(-10_i128), Dec256::new(2), I256::from(-5_i128)),
                    (I256::from(-9_i128), Dec256::new(2), I256::from(-4_i128)),
                    (I256::from(-11_i128), Dec256::new(2), I256::from(-5_i128)),

                    (I256::from(10_i128), Dec256::new(-2), I256::from(-5_i128)),
                    (I256::from(9_i128), Dec256::new(-2), I256::from(-4_i128)),
                    (I256::from(11_i128), Dec256::new(-2), I256::from(-5_i128)),

                    (I256::from(-10_i128), Dec256::new(-2), I256::from(5_i128)),
                    (I256::from(-9_i128), Dec256::new(-2), I256::from(4_i128)),
                    (I256::from(-11_i128), Dec256::new(-2), I256::from(5_i128)),
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_div_dec(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }

            let _0d = Dec::<_, 18>::ZERO;
            let base = bt(_0, Int::TEN);
            assert!(matches!(base.checked_div_dec(_0d), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( checked_div_dec_floor
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new_percent(50), 20_u128),
                    (10_u128, Udec128::new(2), 5_u128),
                    (9_u128, Udec128::new(2), 4_u128),
                    (11_u128, Udec128::new(2), 5_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new(2), U256::from(5_u128)),
                    (U256::from(9_u128), Udec256::new(2), U256::from(4_u128)),
                    (U256::from(11_u128), Udec256::new(2), U256::from(5_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new_percent(50), 20_i128),
                    (10_i128, Dec128::new(2), 5_i128),
                    (9_i128, Dec128::new(2), 4_i128),
                    (11_i128, Dec128::new(2), 5_i128),

                    (-10_i128, Dec128::new(2), -5_i128),
                    (-9_i128, Dec128::new(2), -5_i128),
                    (-11_i128, Dec128::new(2), -6_i128),

                    (10_i128, Dec128::new(-2), -5_i128),
                    (9_i128, Dec128::new(-2), -5_i128),
                    (11_i128, Dec128::new(-2), -6_i128),

                    (-10_i128, Dec128::new(-2), 5_i128),
                    (-9_i128, Dec128::new(-2), 4_i128),
                    (-11_i128, Dec128::new(-2), 5_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new(2), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new(2), I256::from(4_i128)),
                    (I256::from(11_i128), Dec256::new(2), I256::from(5_i128)),

                    (I256::from(-10_i128), Dec256::new(2), I256::from(-5_i128)),
                    (I256::from(-9_i128), Dec256::new(2), I256::from(-5_i128)),
                    (I256::from(-11_i128), Dec256::new(2), I256::from(-6_i128)),

                    (I256::from(10_i128), Dec256::new(-2), I256::from(-5_i128)),
                    (I256::from(9_i128), Dec256::new(-2), I256::from(-5_i128)),
                    (I256::from(11_i128), Dec256::new(-2), I256::from(-6_i128)),

                    (I256::from(-10_i128), Dec256::new(-2), I256::from(5_i128)),
                    (I256::from(-9_i128), Dec256::new(-2), I256::from(4_i128)),
                    (I256::from(-11_i128), Dec256::new(-2), I256::from(5_i128)),
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_div_dec_floor(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }

            let _0d = Dec::<_, 18>::ZERO;
            let base = bt(_0, Int::TEN);
            assert!(matches!(base.checked_div_dec_floor(_0d), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( checked_div_dec_ceil
        inputs = {
            u128 = {
                passing: [
                    (10_u128, Udec128::new_percent(50), 20_u128),
                    (10_u128, Udec128::new(2), 5_u128),
                    (9_u128, Udec128::new(2), 5_u128),
                    (11_u128, Udec128::new(2), 6_u128),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(10_u128), Udec256::new_percent(50), U256::from(20_u128)),
                    (U256::from(10_u128), Udec256::new(2), U256::from(5_u128)),
                    (U256::from(9_u128), Udec256::new(2), U256::from(5_u128)),
                    (U256::from(11_u128), Udec256::new(2), U256::from(6_u128)),
                ]
            }
            i128 = {
                passing: [
                    (10_i128, Dec128::new_percent(50), 20_i128),
                    (10_i128, Dec128::new(2), 5_i128),
                    (9_i128, Dec128::new(2), 5_i128),
                    (11_i128, Dec128::new(2), 6_i128),

                    (-10_i128, Dec128::new(2), -5_i128),
                    (-9_i128, Dec128::new(2), -4_i128),
                    (-11_i128, Dec128::new(2), -5_i128),

                    (10_i128, Dec128::new(-2), -5_i128),
                    (9_i128, Dec128::new(-2), -4_i128),
                    (11_i128, Dec128::new(-2), -5_i128),

                    (-10_i128, Dec128::new(-2), 5_i128),
                    (-9_i128, Dec128::new(-2), 5_i128),
                    (-11_i128, Dec128::new(-2), 6_i128),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(10_i128), Dec256::new_percent(50), I256::from(20_i128)),
                    (I256::from(10_i128), Dec256::new(2), I256::from(5_i128)),
                    (I256::from(9_i128), Dec256::new(2), I256::from(5_i128)),
                    (I256::from(11_i128), Dec256::new(2), I256::from(6_i128)),

                    (I256::from(-10_i128), Dec256::new(2), I256::from(-5_i128)),
                    (I256::from(-9_i128), Dec256::new(2), I256::from(-4_i128)),
                    (I256::from(-11_i128), Dec256::new(2), I256::from(-5_i128)),

                    (I256::from(10_i128), Dec256::new(-2), I256::from(-5_i128)),
                    (I256::from(9_i128), Dec256::new(-2), I256::from(-4_i128)),
                    (I256::from(11_i128), Dec256::new(-2), I256::from(-5_i128)),

                    (I256::from(-10_i128), Dec256::new(-2), I256::from(5_i128)),
                    (I256::from(-9_i128), Dec256::new(-2), I256::from(5_i128)),
                    (I256::from(-11_i128), Dec256::new(-2), I256::from(6_i128)),
                ]
            }
        }
        method = |_0, passing| {
            for (base, dec, expected) in passing {
                let base = bt(_0, Int::new(base));
                let result = base.checked_div_dec_ceil(dec).unwrap();
                assert_eq!(result, Int::new(expected));
            }

            let _0d = Dec::<_, 18>::ZERO;
            let base = bt(_0, Int::TEN);
            assert!(matches!(base.checked_div_dec_ceil(_0d), Err(MathError::DivisionByZero { .. })));
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use {
        crate::{Dec, MultiplyFraction, dec_test, dts, test_utils::dec},
        std::str::FromStr,
    };

    /// Decimals with padding zeros
    fn dec_p<U, const S: u32>(n: &str, d: &str) -> Dec<U, S>
    where
        Dec<U, S>: FromStr,
        <Dec<U, S> as FromStr>::Err: std::fmt::Debug,
    {
        let padding_zero = S as usize - d.len();
        let mut s = String::new();
        s.push_str(n);
        s.push('.');
        s.push_str(&"0".repeat(padding_zero));
        s.push_str(d);

        Dec::<U, S>::from_str(&s).unwrap()
    }

    dec_test!( checked_mul_dec_ceil_floor
        inputs = {
            udec128 = {
                passing: [
                    (dec_p("0", "123"), dec("0.2"), dec_p("0", "24")),
                    (dec_p("0", "567"), dec("0.3"), dec_p("0", "170")),
                    (dec_p("10", "567"), dec("0.3"), dec_p("3", "170")),
                    (dec_p("15", "123456"), dec_p("7","654321"), dec_p("105", "10679007")),
                ]
            }
            udec256 = {
                passing: [
                    (dec_p("0", "123"), dec("0.2"), dec_p("0", "24")),
                    (dec_p("0", "567"), dec("0.3"), dec_p("0", "170")),
                    (dec_p("10", "567"), dec("0.3"), dec_p("3", "170")),
                    (dec_p("15", "123456"), dec_p("7","654321"), dec_p("105", "10679007")),
                ]
            }
            dec128 = {
                passing: [
                    (dec_p("-0", "123"), dec("-0.2"), dec_p("0", "24")),
                    (dec_p("-0", "567"), dec("0.3"), dec_p("-0", "171")),
                    (dec_p("10", "567"), dec("-0.3"), dec_p("-3", "171")),
                    (dec_p("-15", "123456"), dec_p("-7","654321"), dec_p("105", "10679007")),
                ]
            }
            dec256 = {
                passing: [
                    (dec_p("-0", "123"), dec("-0.2"), dec_p("0", "24")),
                    (dec_p("-0", "567"), dec("0.3"), dec_p("-0", "171")),
                    (dec_p("10", "567"), dec("-0.3"), dec_p("-3", "171")),
                    (dec_p("-15", "123456"), dec_p("-7","654321"), dec_p("105", "10679007")),
                ]
            }
        }
        method = |_0: Dec<_, 18>, passing| {
            for (a, b, expected) in passing {
                dts!(_0, a, b, expected);

                // floor
                let result = a.checked_mul_dec_floor(b).unwrap();
                assert_eq!(result, expected);

                // ceil
                let result = a.checked_mul_dec_ceil(b).unwrap();
                assert_eq!(result, expected + dec_p("0", "1"));

            }
        }
    );

    dec_test!( checked_div_dec_floor_ceil
        inputs = {
            udec128 = {
                passing: [
                    (dec_p("0", "123"), dec("2.1"), dec_p("0", "58")),
                    (dec_p("0", "567"), dec("3.3"), dec_p("0", "171")),
                    (dec_p("10", "567"), dec("0.3"), dec("33.333333333333335223")),
                    (dec_p("15", "80"), dec_p("1","30"), dec("14.999999999999999630")),
                ]
            }
            udec256 = {
                passing: [
                    (dec_p("0", "123"), dec("2.1"), dec_p("0", "58")),
                    (dec_p("0", "567"), dec("3.3"), dec_p("0", "171")),
                    (dec_p("10", "567"), dec("0.3"), dec("33.333333333333335223")),
                    (dec_p("15", "80"), dec_p("1","30"), dec("14.999999999999999630")),
                ]
            }
            dec128 = {
                passing: [
                    (dec_p("0", "123"), dec("2.1"), dec_p("0", "58")),
                    (dec_p("-0", "567"), dec("3.3"), dec_p("-0", "172")),
                    (dec_p("10", "567"), dec("-0.3"), dec("-33.333333333333335224")),
                    (dec_p("-15", "80"), dec_p("-1","30"), dec("14.999999999999999630")),
                ]
            }
            dec256 = {
                passing: [
                    (dec_p("0", "123"), dec("2.1"), dec_p("0", "58")),
                    (dec_p("-0", "567"), dec("3.3"), dec_p("-0", "172")),
                    (dec_p("10", "567"), dec("-0.3"), dec("-33.333333333333335224")),
                    (dec_p("-15", "80"), dec_p("-1","30"), dec("14.999999999999999630")),
                ]
            }
        }
        method = |_0: Dec<_, 18>, passing| {
            for (a, b, expected) in passing {
                dts!(_0, a, b, expected);

                // floor
                let result = a.checked_div_dec_floor(b).unwrap();
                assert_eq!(result, expected);

                // ceil
                let result = a.checked_div_dec_ceil(b).unwrap();
                assert_eq!(result, expected + dec_p("0", "1"));
            }
        }
    );
}
