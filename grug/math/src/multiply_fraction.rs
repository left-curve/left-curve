use crate::{Dec, Fraction, Int, IsZero, MathError, MathResult, MultiplyRatio, NumberConst};

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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
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
