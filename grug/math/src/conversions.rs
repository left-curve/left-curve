use crate::{Dec, FixedPoint, Int, MathError, MathResult, Number};

// -------------------------------- int -> dec ---------------------------------

impl<U> Int<U>
where
    Self: Number + Copy + ToString,
    Dec<U>: FixedPoint<U>,
{
    pub fn checked_into_dec(self) -> MathResult<Dec<U>> {
        self.checked_mul(Dec::<U>::DECIMAL_FRACTION)
            .map(Dec::raw)
            .map_err(|_| MathError::overflow_conversion::<_, Dec<U>>(self))
    }
}

// -------------------------------- dec -> int ---------------------------------

impl<U> Dec<U>
where
    Self: FixedPoint<U>,
    Int<U>: Number,
{
    pub fn into_int(self) -> Int<U> {
        // We know the decimal fraction is non-zero, so safe to unwrap.
        self.0.checked_div(Self::DECIMAL_FRACTION).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::{
            int_test, test_utils::bt, Dec128, Dec256, FixedPoint, Int, Int256, MathError,
            NumberConst, Udec128, Udec256, Uint256,
        },
        bnum::types::{I256, U256},
    };

    int_test!( int_to_dec
        inputs = {
            u128 = {
                passing: [
                    (0_u128, Udec128::ZERO),
                    (10, Udec128::TEN),
                    (u128::MAX / 10_u128.pow(Udec128::DECIMAL_PLACES), Udec128::new(u128::MAX / 10_u128.pow(Udec128::DECIMAL_PLACES))),
                ],
                failing: [
                    u128::MAX / 10_u128.pow(Udec128::DECIMAL_PLACES) + 1,
                ]
            }
            u256 = {
                passing: [
                    (U256::ZERO, Udec256::ZERO),
                    (U256::TEN, Udec256::TEN),
                    (U256::MAX / U256::TEN.pow(Udec128::DECIMAL_PLACES), Udec256::raw(Uint256::new(U256::MAX / U256::TEN.pow(Udec128::DECIMAL_PLACES) * U256::TEN.pow(Udec128::DECIMAL_PLACES)))),
                ],
                failing: [
                    U256::MAX / U256::TEN.pow(Udec128::DECIMAL_PLACES) + 1,
                ]
            }
            i128 = {
                passing: [
                    (0_i128, Dec128::ZERO),
                    (10, Dec128::TEN),
                    (-10, -Dec128::TEN),
                    (i128::MAX / 10_i128.pow(Dec128::DECIMAL_PLACES), Dec128::new(i128::MAX / 10_i128.pow(Dec128::DECIMAL_PLACES))),
                    (i128::MIN / 10_i128.pow(Dec128::DECIMAL_PLACES), Dec128::new(i128::MIN / 10_i128.pow(Dec128::DECIMAL_PLACES))),
                ],
                failing: [
                    i128::MAX / 10_i128.pow(Dec128::DECIMAL_PLACES) + 1,
                    i128::MIN / 10_i128.pow(Dec128::DECIMAL_PLACES) - 1,
                ]
            }
            i256 = {
                passing: [
                    (I256::ZERO, Dec256::ZERO),
                    (I256::TEN, Dec256::TEN),
                    (-I256::TEN, -Dec256::TEN),
                    (I256::MAX / I256::TEN.pow(Dec256::DECIMAL_PLACES), Dec256::raw(Int256::new(I256::MAX / I256::TEN.pow(Dec256::DECIMAL_PLACES) * I256::TEN.pow(Dec256::DECIMAL_PLACES)))),
                    (I256::MIN / I256::TEN.pow(Dec256::DECIMAL_PLACES), Dec256::raw(Int256::new(I256::MIN / I256::TEN.pow(Dec256::DECIMAL_PLACES) * I256::TEN.pow(Dec256::DECIMAL_PLACES)))),
                ],
                failing: [
                    I256::MAX / I256::TEN.pow(Dec256::DECIMAL_PLACES) + I256::ONE,
                    I256::MIN / I256::TEN.pow(Dec256::DECIMAL_PLACES) - I256::ONE,
                ]
            }
        }
        method = |_0, samples, failing_samples| {
            for (unsigned, expected) in samples {
                let uint = bt(_0, Int::new(unsigned));
                assert_eq!(uint.checked_into_dec().unwrap(), expected);
            }

            for unsigned in failing_samples {
                let uint = bt(_0, Int::new(unsigned));
                assert!(matches!(uint.checked_into_dec(), Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}
