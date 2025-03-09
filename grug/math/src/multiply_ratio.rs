use crate::{Int, MathResult, NextNumber, Number, NumberConst, PrevNumber, Sign};

/// Describes operations where a number is multiplied by a numerator then
/// immediately divided by a denominator.
///
/// This is different from applying a multiplication and a division sequentially,
/// because the multiplication part can overflow.
pub trait MultiplyRatio: Sized + Copy {
    /// In case the result is non-integer, it is _truncated_; in other words,
    /// _rounded towards zero_:
    /// - positive result: `5 * 3 / 2 = 7.5 => 7`
    /// - negative result: `-5 * 3 / 2 = -7.5 => -7`
    fn checked_multiply_ratio(self, numerator: Self, denominator: Self) -> MathResult<Self>;

    /// In case the result is non-integer, it is _floored_; in other words,
    /// _rounded towards negative infinity_:
    /// - positive result: `5 * 3 / 2 = 7.5 => 7`
    /// - negative result: `-5 * 3 / 2 = -7.5 => -8`
    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self>;

    /// In case the result is non-integer, it is _ceiled_; in other words,
    /// _rounded towards positive infinity_:
    /// - positive result: `5 * 3 / 2 = 7.5 => 8`
    /// - negative result: `-5 * 3 / 2 = -7.5 => -7`
    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self>;

    #[inline]
    fn checked_multiply_ratio_assign(
        &mut self,
        numerator: Self,
        denominator: Self,
    ) -> MathResult<()> {
        *self = self.checked_multiply_ratio(numerator, denominator)?;
        Ok(())
    }

    #[inline]
    fn checked_multiply_ratio_floor_assign(
        &mut self,
        numerator: Self,
        denominator: Self,
    ) -> MathResult<()> {
        *self = self.checked_multiply_ratio_floor(numerator, denominator)?;
        Ok(())
    }

    #[inline]
    fn checked_multiply_ratio_ceil_assign(
        &mut self,
        numerator: Self,
        denominator: Self,
    ) -> MathResult<()> {
        *self = self.checked_multiply_ratio_ceil(numerator, denominator)?;
        Ok(())
    }
}

impl<U> MultiplyRatio for Int<U>
where
    Int<U>: NextNumber + Number + Copy + Sign,
    <Int<U> as NextNumber>::Next:
        Number + NumberConst + Sign + Copy + PartialEq + PrevNumber<Prev = Int<U>>,
{
    fn checked_multiply_ratio(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        self.checked_full_mul(numerator)?
            .checked_div(denominator.into_next())?
            .checked_into_prev()
    }

    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        let dividend = self.checked_full_mul(numerator)?;
        let denominator = denominator.into_next();
        let mut res = dividend.checked_div(denominator)?;

        // If the result is a negative non-integer, we floor it by subtracting 1.
        // Otherwise, simply return the result.
        if (self.is_positive() == numerator.is_positive()) != denominator.is_positive()
            && res.checked_mul(denominator)? != dividend
        {
            res = res.checked_sub(<Self as NextNumber>::Next::ONE)?;
        }

        res.checked_into_prev()
    }

    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        let dividend = self.checked_full_mul(numerator)?;
        let denominator = denominator.into_next();
        let mut res = dividend.checked_div(denominator)?;

        // If the result is a positive non-integer, we ceil it by adding 1.
        // Otherwise, simply return the result.
        if ((self.is_positive() == numerator.is_positive()) == denominator.is_positive())
            && res.checked_mul(denominator)? != dividend
        {
            res = res.checked_add(<Self as NextNumber>::Next::ONE)?;
        }

        res.checked_into_prev()
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Int, MathError, MultiplyRatio, NumberConst, dts, int_test},
        bnum::types::{I256, U256},
    };

    int_test!( multiply_ratio
        inputs = {
            u128 = {
                passing: [
                    (500_u128, 3_u128, 3_u128, 500_u128),
                    (500_u128, 3_u128, 2_u128, 750_u128),
                    (500_u128, 333333_u128, 222222_u128, 750_u128),
                    (500_u128, 2_u128, 3_u128, 333_u128),
                    (500_u128, 222222_u128, 333333_u128, 333_u128),
                    (500_u128, 5_u128, 6_u128, 416_u128),
                    (500_u128, 100_u128, 120_u128, 416_u128),
                    (u128::MAX, u128::MAX, u128::MAX, u128::MAX),
                ]
            }
            u256 = {
                passing: [
                    (U256::from(500_u128), U256::ONE, U256::ONE, U256::from(500_u128)),
                    (U256::from(500_u128), U256::from(3_u128), U256::from(2_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(333333_u128), U256::from(222222_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(2_u128), U256::from(3_u128), U256::from(333_u128)),
                    (U256::from(500_u128), U256::from(222222_u128), U256::from(333333_u128), U256::from(333_u128)),
                    (U256::from(500_u128), U256::from(5_u128), U256::from(6_u128), U256::from(416_u128)),
                    (U256::from(500_u128), U256::from(100_u128), U256::from(120_u128), U256::from(416_u128)),
                    (U256::MAX, U256::MAX, U256::MAX, U256::MAX),
                ]
            }
            i128 = {
                passing: [
                    (500_i128, 3_i128, 3_i128, 500_i128),
                    (500_i128, 3_i128, 2_i128, 750_i128),
                    (500_i128, 333333_i128, 222222_i128, 750_i128),
                    (500_i128, 2_i128, 3_i128, 333_i128),
                    (500_i128, 222222_i128, 333333_i128, 333_i128),
                    (500_i128, 5_i128, 6_i128, 416_i128),
                    (500_i128, 100_i128, 120_i128, 416_i128),
                    (i128::MAX, i128::MAX, i128::MAX, i128::MAX),

                    (500_i128, -2_i128, 3_i128, -333_i128),
                    (500_i128, 2_i128, -3_i128, -333_i128),
                    (500_i128, -2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, 3_i128, 333_i128),
                    (-500_i128, 2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, -3_i128, -333_i128),
                    (i128::MIN, 2, 2, i128::MIN),
                ]
            }
            i256 = {
                passing: [
                    (I256::from(500_i128), I256::ONE, I256::ONE, I256::from(500_i128)),
                    (I256::from(500_i128), I256::from(3_i128), I256::from(2_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(333333_i128), I256::from(222222_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(3_i128), I256::from(333_i128)),
                    (I256::from(500_i128), I256::from(222222_i128), I256::from(333333_i128), I256::from(333_i128)),
                    (I256::from(500_i128), I256::from(5_i128), I256::from(6_i128), I256::from(416_i128)),
                    (I256::from(500_i128), I256::from(100_i128), I256::from(120_i128), I256::from(416_i128)),
                    (I256::MAX, I256::MAX, I256::MAX, I256::MAX),

                    (I256::from(500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(-333_i128)),
                    (I256::MIN, I256::from(2_i128), I256::from(2_i128), I256::MIN),
                ]
            }
        }
        method = |_0, passing| {
            for (base, numerator, denominator, expected) in passing {
                let base = Int::new(base);
                let numerator = Int::new(numerator);
                let denominator = Int::new(denominator);
                let expected = Int::new(expected);
                dts!(_0, base, numerator, denominator, expected);
                assert_eq!(base.checked_multiply_ratio(numerator, denominator).unwrap(), expected);
            }

            // 0 / x = 0
            let _1 = Int::ONE;
            let _10 = Int::TEN;
            dts!(_0, _1, _10);
            assert_eq!(_0.checked_multiply_ratio(_1, _10).unwrap(), _0);

            // Not overflow
            let max = Int::MAX;
            assert_eq!(max.checked_multiply_ratio(_10, _10).unwrap(), max);

            // Divison by zero
            assert!(matches!(max.checked_multiply_ratio(_10, _0), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( multiply_ratio_floor
        inputs = {
            u128 = {
                passing: [
                    (1_u128, 1_u128, 10_u128, 0_u128),
                    (0_u128, 1_u128, 10_u128, 0_u128),
                    (1_u128, 0_u128, 10_u128, 0_u128),
                    (500_u128, 3_u128, 3_u128, 500_u128),
                    (500_u128, 3_u128, 2_u128, 750_u128),
                    (500_u128, 333333_u128, 222222_u128, 750_u128),
                    (500_u128, 2_u128, 3_u128, 333_u128),
                    (500_u128, 222222_u128, 333333_u128, 333_u128),
                    (500_u128, 5_u128, 6_u128, 416_u128),
                    (500_u128, 100_u128, 120_u128, 416_u128),
                    (u128::MAX, u128::MAX, u128::MAX, u128::MAX),
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, U256::ONE, U256::TEN, U256::ZERO),
                    (U256::ONE, U256::ZERO, U256::TEN, U256::ZERO),
                    (U256::ONE, U256::ONE, U256::TEN, U256::ZERO),
                    (U256::from(500_u128), U256::ONE, U256::ONE, U256::from(500_u128)),
                    (U256::from(500_u128), U256::from(3_u128), U256::from(2_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(333333_u128), U256::from(222222_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(2_u128), U256::from(3_u128), U256::from(333_u128)),
                    (U256::from(500_u128), U256::from(222222_u128), U256::from(333333_u128), U256::from(333_u128)),
                    (U256::from(500_u128), U256::from(5_u128), U256::from(6_u128), U256::from(416_u128)),
                    (U256::from(500_u128), U256::from(100_u128), U256::from(120_u128), U256::from(416_u128)),
                    (U256::MAX, U256::MAX, U256::MAX, U256::MAX),
                ]
            }
            i128 = {
                passing: [
                    (1_i128, 0_i128, 10_i128, 0_i128),
                    (0_i128, 1_i128, 10_i128, 0_i128),
                    (1_i128, 1_i128, 10_i128, 0_i128),
                    (500_i128, 3_i128, 3_i128, 500_i128),
                    (500_i128, 3_i128, 2_i128, 750_i128),
                    (500_i128, 333333_i128, 222222_i128, 750_i128),
                    (500_i128, 2_i128, 3_i128, 333_i128),
                    (500_i128, 222222_i128, 333333_i128, 333_i128),
                    (500_i128, 5_i128, 6_i128, 416_i128),
                    (500_i128, 100_i128, 120_i128, 416_i128),
                    (i128::MAX, i128::MAX, i128::MAX, i128::MAX),

                    (-1_i128, 0_i128, 10_i128, 0_i128),
                    (0_i128, -1_i128, 10_i128, 0_i128),
                    (-1_i128, 1_i128, 10_i128, -1_i128),
                    (500_i128, -2_i128, 3_i128, -334_i128),
                    (500_i128, 2_i128, -3_i128, -334_i128),
                    (500_i128, -2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, 3_i128, 333_i128),
                    (-500_i128, 2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, -3_i128, -334_i128),
                    (i128::MIN, 2, 2, i128::MIN)
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, I256::ONE, I256::TEN, I256::ZERO),
                    (I256::ONE, I256::ZERO, I256::TEN, I256::ZERO),
                    (I256::ONE, I256::ONE, I256::TEN, I256::ZERO),
                    (I256::from(500_i128), I256::ONE, I256::ONE, I256::from(500_i128)),
                    (I256::from(500_i128), I256::from(3_i128), I256::from(2_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(333333_i128), I256::from(222222_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(3_i128), I256::from(333_i128)),
                    (I256::from(500_i128), I256::from(222222_i128), I256::from(333333_i128), I256::from(333_i128)),
                    (I256::from(500_i128), I256::from(5_i128), I256::from(6_i128), I256::from(416_i128)),
                    (I256::from(500_i128), I256::from(100_i128), I256::from(120_i128), I256::from(416_i128)),
                    (I256::MAX, I256::MAX, I256::MAX, I256::MAX),

                    (I256::ZERO, -I256::ONE, I256::TEN, I256::ZERO),
                    (-I256::ONE, I256::ZERO, I256::TEN, I256::ZERO),
                    (-I256::ONE, I256::ONE, I256::TEN, -I256::ONE),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(-334_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(-334_i128)),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(-334_i128)),
                    (I256::MIN, I256::from(2_i128), I256::from(2_i128), I256::MIN),
                ]
            }
        }
        method = |_0, passing| {
            for (base, numerator, denominator, expected) in passing {
                let base = Int::new(base);
                let numerator = Int::new(numerator);
                let denominator = Int::new(denominator);
                let expected = Int::new(expected);
                dts!(_0, base, numerator, denominator, expected);
                assert_eq!(base.checked_multiply_ratio_floor(numerator, denominator).unwrap(), expected);
            }

            // 0 / x = 0
            let _1 = Int::ONE;
            let _10 = Int::TEN;
            dts!(_0, _1, _10);
            assert_eq!(_0.checked_multiply_ratio_floor(_1, _10).unwrap(), _0);

            // Not overflow
            let max = Int::MAX;
            let min = Int::MIN;
            assert_eq!(max.checked_multiply_ratio_floor(_10, _10).unwrap(), max);
            assert_eq!(min.checked_multiply_ratio_floor(_10, _10).unwrap(), min);

            // Divison by zero
            assert!(matches!(max.checked_multiply_ratio_floor(_10, _0), Err(MathError::DivisionByZero { .. })));
        }
    );

    int_test!( multiply_ratio_ceil
        inputs = {
            u128 = {
                passing: [
                    (1_u128, 1_u128, 10_u128, 1_u128),
                    (500_u128, 3_u128, 3_u128, 500_u128),
                    (500_u128, 3_u128, 2_u128, 750_u128),
                    (500_u128, 333333_u128, 222222_u128, 750_u128),
                    (500_u128, 2_u128, 3_u128, 334_u128),
                    (500_u128, 222222_u128, 333333_u128, 334_u128),
                    (500_u128, 5_u128, 6_u128, 417_u128),
                    (500_u128, 100_u128, 120_u128, 417_u128),
                    (u128::MAX, u128::MAX, u128::MAX, u128::MAX),
                ]
            }
            u256 = {
                passing: [
                    (U256::ONE, U256::ONE, U256::TEN, U256::ONE),
                    (U256::from(500_u128), U256::ONE, U256::ONE, U256::from(500_u128)),
                    (U256::from(500_u128), U256::from(3_u128), U256::from(2_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(333333_u128), U256::from(222222_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(2_u128), U256::from(3_u128), U256::from(334_u128)),
                    (U256::from(500_u128), U256::from(222222_u128), U256::from(333333_u128), U256::from(334_u128)),
                    (U256::from(500_u128), U256::from(5_u128), U256::from(6_u128), U256::from(417_u128)),
                    (U256::from(500_u128), U256::from(100_u128), U256::from(120_u128), U256::from(417_u128)),
                    (U256::MAX, U256::MAX, U256::MAX, U256::MAX),
                ]
            }
            i128 = {
                passing: [
                    (1_i128, 1_i128, 10_i128, 1_i128),
                    (500_i128, 3_i128, 3_i128, 500_i128),
                    (500_i128, 3_i128, 2_i128, 750_i128),
                    (500_i128, 333333_i128, 222222_i128, 750_i128),
                    (500_i128, 2_i128, 3_i128, 334_i128),
                    (500_i128, 222222_i128, 333333_i128, 334_i128),
                    (500_i128, 5_i128, 6_i128, 417_i128),
                    (500_i128, 100_i128, 120_i128, 417_i128),
                    (i128::MAX, i128::MAX, i128::MAX, i128::MAX),

                    (-1_i128, 1_i128, 10_i128, 0_i128),
                    (500_i128, -2_i128, 3_i128, -333_i128),
                    (500_i128, 2_i128, -3_i128, -333_i128),
                    (500_i128, -2_i128, -3_i128, 334_i128),
                    (-500_i128, -2_i128, 3_i128, 334_i128),
                    (-500_i128, 2_i128, -3_i128, 334_i128),
                    (-500_i128, -2_i128, -3_i128, -333_i128),
                    (i128::MIN, 2, 2, i128::MIN),
                ]
            }
            i256 = {
                passing: [
                    (I256::ONE, I256::ONE, I256::TEN, I256::ONE),
                    (I256::from(500_i128), I256::ONE, I256::ONE, I256::from(500_i128)),
                    (I256::from(500_i128), I256::from(3_i128), I256::from(2_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(333333_i128), I256::from(222222_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(3_i128), I256::from(334_i128)),
                    (I256::from(500_i128), I256::from(222222_i128), I256::from(333333_i128), I256::from(334_i128)),
                    (I256::from(500_i128), I256::from(5_i128), I256::from(6_i128), I256::from(417_i128)),
                    (I256::from(500_i128), I256::from(100_i128), I256::from(120_i128), I256::from(417_i128)),
                    (I256::MAX, I256::MAX, I256::MAX, I256::MAX),

                    (-I256::ONE, I256::ONE, I256::TEN, -I256::ZERO),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(-333_i128)),
                    (I256::MIN, I256::from(2_i128), I256::from(2_i128), I256::MIN),
                ]
            }
        }
        method = |_0, passing| {
            for (base, numerator, denominator, expected) in passing {
                let base = Int::new(base);
                let numerator = Int::new(numerator);
                let denominator = Int::new(denominator);
                let expected = Int::new(expected);
                dts!(_0, base, numerator, denominator, expected);
                assert_eq!(base.checked_multiply_ratio_ceil(numerator, denominator).unwrap(), expected);
            }

            // 0 / x = 0
            let _1 = Int::ONE;
            let _10 = Int::TEN;
            dts!(_0, _1, _10);
            assert_eq!(_0.checked_multiply_ratio_ceil(_1, _10).unwrap(), _0);

            // Not overflow
            let max = Int::MAX;
            assert_eq!(max.checked_multiply_ratio_ceil(_10, _10).unwrap(), max);

            // Divison by zero
            assert!(matches!(max.checked_multiply_ratio_ceil(_10, _0), Err(MathError::DivisionByZero { .. })));
        }
    );
}
