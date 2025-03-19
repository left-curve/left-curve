use crate::{Dec, FixedPoint, IsZero, MathResult, Number, NumberConst, Sign};

/// Describes operations that decimal types must implement, which may not be
/// relevant for non-decimal types.
pub trait Decimal: Sized + Copy {
    fn checked_floor(self) -> MathResult<Self>;

    fn checked_ceil(self) -> MathResult<Self>;

    #[inline]
    fn checked_floor_assign(&mut self) -> MathResult<()> {
        *self = self.checked_floor()?;
        Ok(())
    }

    #[inline]
    fn checked_ceil_assign(&mut self) -> MathResult<()> {
        *self = self.checked_ceil()?;
        Ok(())
    }
}

impl<U, const S: u32> Decimal for Dec<U, S>
where
    Self: FixedPoint<U>,
    U: Number + NumberConst + Sign + IsZero + Copy + PartialEq,
{
    fn checked_floor(self) -> MathResult<Self> {
        let rem = self.0.checked_rem(Self::PRECISION)?;

        match (rem.is_zero(), rem.is_negative()) {
            (false, true) => self.0.checked_sub(Self::PRECISION + rem).map(Self),
            (false, false) => self.0.checked_sub(rem).map(Self),
            (true, _) => Ok(self),
        }
    }

    fn checked_ceil(self) -> MathResult<Self> {
        let rem = self.0.checked_rem(Self::PRECISION)?;

        match (rem.is_zero(), rem.is_negative()) {
            (false, true) => self.0.checked_sub(rem).map(Self),
            (false, false) => self.0.checked_add(Self::PRECISION - rem).map(Self),
            (true, _) => Ok(self),
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{
            Dec, Dec128, Dec256, Decimal, FixedPoint, MathError, NumberConst, Udec128, Udec256,
            dec_test, dts,
            test_utils::{bt, dt},
        },
        std::str::FromStr,
    };

    dec_test!( checked_floor
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::ONE, Udec128::ONE),
                    (Udec128::new_percent(101), Udec128::ONE),
                    (Udec128::new_percent(199), Udec128::ONE),
                    (Udec128::new_percent(200), Udec128::from_str("2").unwrap()),
                ],
                failing: []
            }
            udec256 = {
                passing: [
                    (Udec256::ONE, Udec256::ONE),
                    (Udec256::new_percent(101), Udec256::ONE),
                    (Udec256::new_percent(199), Udec256::ONE),
                    (Udec256::new_percent(200), Udec256::from_str("2").unwrap()),
                ],
                failing: []
            }
            dec128 = {
                passing: [
                    (Dec128::ONE, Dec128::ONE),
                    (Dec128::new_percent(101), Dec128::ONE),
                    (Dec128::new_percent(109), Dec128::ONE),
                    (Dec128::new_percent(200), Dec128::from_str("2").unwrap()),
                    (-Dec128::ONE, -Dec128::ONE),
                    (Dec128::new_percent(-99), -Dec128::ONE),
                    (Dec128::new_percent(-101), Dec128::new(-2)),
                    (Dec128::new_percent(-199), Dec128::new(-2)),
                    (Dec128::new_percent(-200), Dec128::from_str("-2").unwrap()),
                ],
                failing: [
                    Dec128::MIN,
                ]
            }
            dec256 = {
                passing: [
                    (Dec256::ONE, Dec256::ONE),
                    (Dec256::new_percent(101), Dec256::ONE),
                    (Dec256::new_percent(109), Dec256::ONE),
                    (Dec256::new_percent(200), Dec256::from_str("2").unwrap()),
                    (-Dec256::ONE, -Dec256::ONE),
                    (Dec256::new_percent(-99), -Dec256::ONE),
                    (Dec256::new_percent(-101), Dec256::new(-2)),
                    (Dec256::new_percent(-199), Dec256::new(-2)),
                    (Dec256::new_percent(-200), Dec256::from_str("-2").unwrap()),
                ],
                failing: [
                    Dec256::MIN,
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing, failing| {
            for (base, expect) in passing {
                dts!(_0d, base, expect);
                assert_eq!(base.checked_floor().unwrap(), expect);
            }
            for base in failing {
                dt(_0d, base);
                assert!(matches!(base.checked_floor(), Err(MathError::OverflowSub { .. })));
            }
        }
    );

    dec_test!( checked_ceil
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::ONE, Udec128::ONE),
                    (Udec128::new_percent(99), Udec128::ONE),
                    (Udec128::new_percent(101), Udec128::new(2)),
                    (Udec128::new_percent(199), Udec128::new(2)),
                    (Udec128::new_percent(200), Udec128::from_str("2").unwrap()),
                ]
            }
            udec256 = {
                passing: [
                    (Udec256::ONE, Udec256::ONE),
                    (Udec256::new_percent(99), Udec256::ONE),
                    (Udec256::new_percent(101), Udec256::new(2)),
                    (Udec256::new_percent(199), Udec256::new(2)),
                    (Udec256::new_percent(200), Udec256::from_str("2").unwrap()),
                ]
            }
            dec128 = {
                passing: [
                    (Dec128::ONE, Dec128::ONE),
                    (Dec128::new_percent(99), Dec128::ONE),
                    (Dec128::new_percent(101), Dec128::new(2)),
                    (Dec128::new_percent(199), Dec128::new(2)),
                    (Dec128::new_percent(200), Dec128::from_str("2").unwrap()),
                    (-Dec128::ONE, -Dec128::ONE),
                    (Dec128::new_percent(-99), -Dec128::ZERO),
                    (Dec128::new_percent(-101), Dec128::new(-1)),
                    (Dec128::new_percent(-199), Dec128::new(-1)),
                    (Dec128::new_percent(-200), Dec128::from_str("-2").unwrap()),
                    (Dec128::MIN, Dec(Dec128::MIN.0 / Dec128::PRECISION * Dec128::PRECISION)),
                ]
            }
            dec256 = {
                passing: [
                    (Dec256::ONE, Dec256::ONE),
                    (Dec256::new_percent(99), Dec256::ONE),
                    (Dec256::new_percent(101), Dec256::new(2)),
                    (Dec256::new_percent(199), Dec256::new(2)),
                    (Dec256::new_percent(200), Dec256::from_str("2").unwrap()),
                    (-Dec256::ONE, -Dec256::ONE),
                    (Dec256::new_percent(-99), -Dec256::ZERO),
                    (Dec256::new_percent(-101), Dec256::new(-1)),
                    (Dec256::new_percent(-199), Dec256::new(-1)),
                    (Dec256::new_percent(-200), Dec256::from_str("-2").unwrap()),
                    (Dec256::MIN, Dec(Dec256::MIN.0 / Dec256::PRECISION * Dec256::PRECISION)),
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing| {
            for (base, expect) in passing {
                dts!(_0d, base, expect);
                assert_eq!(base.checked_ceil().unwrap(), expect);
            }

            let max = bt(_0d, Dec::MAX);
            assert!(matches!(max.checked_ceil(), Err(MathError::OverflowAdd { .. })));
        }
    );
}
