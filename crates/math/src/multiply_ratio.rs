use crate::{Int, IsZero, MathResult, NextNumber, Number, NumberConst, PrevNumber, Sign};

/// Describes operations where a number is multiplied by a numerator then
/// immediately divided by a denominator.
/// This is different from applying a multiplication and a division sequentially,
/// because the multiplication part can overflow.
pub trait MultiplyRatio: Sized {
    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self>;

    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self>;
}

impl<U> MultiplyRatio for Int<U>
where
    Int<U>: NextNumber + NumberConst + Number + Copy + Sign,
    <Int<U> as NextNumber>::Next: Number + IsZero + Copy + PrevNumber<Prev = Int<U>>,
{
    fn checked_multiply_ratio_floor(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        let dividend = self.checked_full_mul(numerator)?;
        let res = dividend
            .checked_div(denominator.into_next())?
            .checked_into_prev()?;

        if res.is_negative() {
            let remained = dividend.checked_rem(denominator.into_next())?;
            if !remained.is_zero() {
                res.checked_sub(Self::ONE)
            } else {
                Ok(res)
            }
        } else {
            Ok(res)
        }
    }

    fn checked_multiply_ratio_ceil(self, numerator: Self, denominator: Self) -> MathResult<Self> {
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(denominator.into_next())?;

        if !remained.is_zero() {
            floor_result.checked_add(Self::ONE)
        } else {
            Ok(floor_result)
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::{dts, int_test, Int, MathError, MultiplyRatio, NumberConst},
        bnum::types::{I256, U256},
    };

    int_test!( multiply_ratio_floor
        inputs = {
            u128 = {
                passing: [
                    (500_u128, 3_u128, 3_u128, 500_u128),
                    (500_u128, 3_u128, 2_u128, 750_u128),
                    (500_u128, 333333_u128, 222222_u128, 750_u128),
                    (500_u128, 2_u128, 3_u128, 333_u128),
                    (500_u128, 222222_u128, 333333_u128, 333_u128),
                    (500_u128, 5_u128, 6_u128, 416_u128),
                    (500_u128, 100_u128, 120_u128, 416_u128)
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

                    (500_i128, -2_i128, 3_i128, -334_i128),
                    (500_i128, 2_i128, -3_i128, -334_i128),
                    (500_i128, -2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, 3_i128, 333_i128),
                    (-500_i128, 2_i128, -3_i128, 333_i128),
                    (-500_i128, -2_i128, -3_i128, -334_i128)
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

                    (I256::from(500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(-334_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(-334_i128)),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(333_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(-334_i128)),
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
                    (500_u128, 3_u128, 3_u128, 500_u128),
                    (500_u128, 3_u128, 2_u128, 750_u128),
                    (500_u128, 333333_u128, 222222_u128, 750_u128),
                    (500_u128, 2_u128, 3_u128, 334_u128),
                    (500_u128, 222222_u128, 333333_u128, 334_u128),
                    (500_u128, 5_u128, 6_u128, 417_u128),
                    (500_u128, 100_u128, 120_u128, 417_u128)
                ]
            }
            u256 = {
                passing: [
                    (U256::from(500_u128), U256::ONE, U256::ONE, U256::from(500_u128)),
                    (U256::from(500_u128), U256::from(3_u128), U256::from(2_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(333333_u128), U256::from(222222_u128), U256::from(750_u128)),
                    (U256::from(500_u128), U256::from(2_u128), U256::from(3_u128), U256::from(334_u128)),
                    (U256::from(500_u128), U256::from(222222_u128), U256::from(333333_u128), U256::from(334_u128)),
                    (U256::from(500_u128), U256::from(5_u128), U256::from(6_u128), U256::from(417_u128)),
                    (U256::from(500_u128), U256::from(100_u128), U256::from(120_u128), U256::from(417_u128)),
                ]
            }
            i128 = {
                passing: [
                    (500_i128, 3_i128, 3_i128, 500_i128),
                    (500_i128, 3_i128, 2_i128, 750_i128),
                    (500_i128, 333333_i128, 222222_i128, 750_i128),
                    (500_i128, 2_i128, 3_i128, 334_i128),
                    (500_i128, 222222_i128, 333333_i128, 334_i128),
                    (500_i128, 5_i128, 6_i128, 417_i128),
                    (500_i128, 100_i128, 120_i128, 417_i128),

                    (500_i128, -2_i128, 3_i128, -333_i128),
                    (500_i128, 2_i128, -3_i128, -333_i128),
                    (500_i128, -2_i128, -3_i128, 334_i128),
                    (-500_i128, -2_i128, 3_i128, 334_i128),
                    (-500_i128, 2_i128, -3_i128, 334_i128),
                    (-500_i128, -2_i128, -3_i128, -333_i128)
                ]
            }
            i256 = {
                passing: [
                    (I256::from(500_i128), I256::ONE, I256::ONE, I256::from(500_i128)),
                    (I256::from(500_i128), I256::from(3_i128), I256::from(2_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(333333_i128), I256::from(222222_i128), I256::from(750_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(3_i128), I256::from(334_i128)),
                    (I256::from(500_i128), I256::from(222222_i128), I256::from(333333_i128), I256::from(334_i128)),
                    (I256::from(500_i128), I256::from(5_i128), I256::from(6_i128), I256::from(417_i128)),
                    (I256::from(500_i128), I256::from(100_i128), I256::from(120_i128), I256::from(417_i128)),

                    (I256::from(500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(-333_i128)),
                    (I256::from(500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(2_i128), I256::from(-3_i128), I256::from(334_i128)),
                    (I256::from(-500_i128), I256::from(-2_i128), I256::from(-3_i128), I256::from(-333_i128)),
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
            // let min = Int::MIN;
            assert_eq!(max.checked_multiply_ratio_ceil(_10, _10).unwrap(), max);
            // assert_eq!(min.checked_multiply_ratio_ceil(_10, _10).unwrap(), min);

            // Divison by zero
            assert!(matches!(max.checked_multiply_ratio_ceil(_10, _0), Err(MathError::DivisionByZero { .. })));
        }
    );
}
