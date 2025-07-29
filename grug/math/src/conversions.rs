use {
    crate::{
        Dec, Exponentiate, FixedPoint, Int, IsZero, MathError, MathResult, Number, NumberConst,
        Sign,
    },
    std::cmp::Ordering,
};

// -------------------------------- int -> dec ---------------------------------

impl<U> Int<U>
where
    Self: Number + ToString,
{
    pub fn checked_into_dec<const S: u32>(self) -> MathResult<Dec<U, S>>
    where
        Dec<U, S>: FixedPoint<U>,
    {
        self.checked_mul(Dec::<U, S>::PRECISION)
            .map(Dec::raw)
            .map_err(|_| MathError::overflow_conversion::<_, Dec<U, S>>(self))
    }
}

// -------------------------------- dec -> int ---------------------------------

impl<U, const S: u32> Dec<U, S>
where
    Self: FixedPoint<U>,
    Int<U>: Number + NumberConst + Sign + IsZero,
{
    /// Convert the decimal number to an integer, rounded towards zero.
    pub fn into_int(self) -> Int<U> {
        // Safe to unwrap because we know `Self::PRECISION` is non-zero.
        self.0.checked_div(Self::PRECISION).unwrap()
    }

    /// Convert the decimal number to an integer, rounded towards negative infinity.
    pub fn into_int_floor(self) -> Int<U> {
        // Safe to unwrap because we know `Self::PRECISION` is non-zero.
        let rem = self.0.checked_rem(Self::PRECISION).unwrap();
        let int = self.into_int();

        match (rem.is_zero(), rem.is_negative()) {
            (true, _) | (false, false) => int,
            // Safe to unwrap, because the biggest value supported by `Int<U>`
            // is necessarily more than one unit bigger than that by `Dec<U, S>`.
            (false, true) => int.checked_sub(Int::<U>::ONE).unwrap(),
        }
    }

    /// Convert the decimal number to an integer, rounded towards positive infinity.
    pub fn into_int_ceil(self) -> Int<U> {
        // Safe to unwrap because we know `Self::PRECISION` is non-zero.
        let rem = self.0.checked_rem(Self::PRECISION).unwrap();
        let int = self.into_int();

        match (rem.is_zero(), rem.is_negative()) {
            (true, _) | (false, true) => int,
            // Safe to unwrap, because the smallest value supported by `Int<U>`
            // is necessarily more than one unit smaller than that by `Dec<U, S>`.
            (false, false) => int.checked_add(Int::<U>::ONE).unwrap(),
        }
    }
}

// -------------------------- dec<U, S> -> dec<U, S1> --------------------------

impl<U, const S: u32> Dec<U, S>
where
    U: Number + Exponentiate + NumberConst,
{
    pub fn convert_precision<const S1: u32>(self) -> MathResult<Dec<U, S1>> {
        match S.cmp(&S1) {
            Ordering::Less => {
                let diff = S1 - S;
                Ok(Dec::raw(
                    self.0.checked_mul(Int::<U>::TEN.checked_pow(diff)?)?,
                ))
            },
            Ordering::Equal => Ok(Dec::raw(self.0)),
            Ordering::Greater => {
                let diff = S - S1;
                Ok(Dec::raw(
                    self.0.checked_div(Int::<U>::TEN.checked_pow(diff)?)?,
                ))
            },
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod int_tests {
    use {
        crate::{
            Dec, Dec128, Dec256, Int, Int256, MathError, MathResult, NumberConst, Udec128, Udec256,
            Uint256, int_test, test_utils::bt,
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
                let dec: MathResult<Dec<_, 18>> = uint.checked_into_dec();
                assert!(matches!(dec, Err(MathError::OverflowConversion { .. })));
            }
        }
    );
}

#[cfg(test)]
mod dec_tests {
    use {
        crate::{
            Dec, Dec128, Dec256, FixedPoint, Int, Int128, IsZero, Number, NumberConst, Sign,
            Udec128, Udec256, Uint128, dec_test,
            test_utils::{bt, dec, dt},
        },
        bnum::types::{I256, U256},
        std::str::FromStr,
        test_case::test_case,
    };

    dec_test!( dec_to_int
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::ZERO, u128::ZERO),
                    (Udec128::MIN, u128::ZERO),
                    (Udec128::new_percent(101), 1),
                    (Udec128::new_percent(199), 1),
                    (Udec128::new(2), 2),
                    (Udec128::MAX, u128::MAX / Udec128::PRECISION.0),
                ]
            }
            udec256 = {
                passing: [
                    (Udec256::ZERO, U256::ZERO),
                    (Udec256::MIN, U256::ZERO),
                    (Udec256::new_percent(101), U256::ONE),
                    (Udec256::new_percent(199), U256::ONE),
                    (Udec256::new(2), U256::from(2_u128)),
                    (Udec256::MAX, U256::MAX / Udec256::PRECISION.0),
                ]
            }
            dec128 = {
                passing: [
                    (Dec128::ZERO, i128::ZERO),
                    (Dec128::MIN, i128::MIN / Dec128::PRECISION.0),
                    (Dec128::new_percent(101), 1),
                    (Dec128::new_percent(199), 1),
                    (Dec128::new(2), 2),
                    (Dec128::new_percent(-101), -1),
                    (Dec128::new_percent(-199), -1),
                    (Dec128::new(-2), -2),
                    (Dec128::MAX, i128::MAX / Dec128::PRECISION.0),
                ]
            }
            dec256 = {
                passing: [
                    (Dec256::ZERO, I256::ZERO),
                    (Dec256::MIN, I256::MIN / Dec256::PRECISION.0),
                    (Dec256::new_percent(101), I256::ONE),
                    (Dec256::new_percent(199), I256::ONE),
                    (Dec256::new(2), I256::from(2)),
                    (Dec256::new_percent(-101), -I256::ONE),
                    (Dec256::new_percent(-199), -I256::ONE),
                    (Dec256::new(-2), I256::from(-2)),
                    (Dec256::MAX, I256::MAX / Dec256::PRECISION.0),
                ]
            }
        }
        method = |_0d: Dec<_, 18>, samples| {
            for (dec, expected) in samples {
                let expected = bt(_0d.0, Int::new(expected));
                dt(_0d, dec);
                assert_eq!(dec.into_int(), expected);
            }
        }
    );

    #[test]
    fn convert_precision() {
        let dec_18: Udec128 = dec("123.123456789012345678");
        let dec_6 = dec_18.convert_precision::<6>().unwrap();
        assert_eq!(dec_6, dec("123.123456"));

        // Try at max
        let dec_18 = Udec128::MAX;
        dec_18.checked_add(Udec128::TICK).unwrap_err();
        let dec_6 = dec_18.convert_precision::<6>().unwrap();
        dec_6.checked_add(Dec::<_, 6>::TICK).unwrap();
    }

    #[test_case(
        Udec128::ZERO
        => (Uint128::ZERO, Uint128::ZERO, Uint128::ZERO);
        "unsigned zero"
    )]
    #[test_case(
        Udec128::ONE
        => (Uint128::ONE, Uint128::ONE, Uint128::ONE);
        "unsigned positive one"
    )]
    #[test_case(
        Udec128::from_str("1.5").unwrap()
        => (Uint128::ONE, Uint128::ONE, Uint128::new(2));
        "unsigned positive one point five"
    )]
    #[test_case(
        Udec128::MAX // (2^128 - 1) / 10^18 = 340,282,366,920,938,463,463.374607431768211455
        => (
            Uint128::new(340_282_366_920_938_463_463),
            Uint128::new(340_282_366_920_938_463_463),
            Uint128::new(340_282_366_920_938_463_464), // rounded up by 1
        );
        "unsigned positive max"
    )]
    #[test_case(
        Dec128::ZERO
        => (Int128::ZERO, Int128::ZERO, Int128::ZERO);
        "signed zero"
    )]
    #[test_case(
        Dec128::ONE
        => (Int128::ONE, Int128::ONE, Int128::ONE);
        "signed positive one"
    )]
    #[test_case(
        Dec128::from_str("1.5").unwrap()
        => (Int128::ONE, Int128::ONE, Int128::new(2));
        "signed positive one point five"
    )]
    #[test_case(
        Dec128::MAX // (2^127 - 1) / 10^18 = 170,141,183,460,469,231,731.687303715884105727
        => (
            Int128::new(170_141_183_460_469_231_731),
            Int128::new(170_141_183_460_469_231_731),
            Int128::new(170_141_183_460_469_231_732), // rounded up by 1
        );
        "signed positive max"
    )]
    #[test_case(
        -Dec128::ONE
        => (-Int128::ONE, -Int128::ONE, -Int128::ONE);
        "signed negative one"
    )]
    #[test_case(
        Dec128::from_str("-1.5").unwrap()
        => (
            -Int128::ONE,    // rounded towards zero
            -Int128::new(2), // rounded down by 1
            -Int128::ONE,    // rounded up by 1
        );
        "signed negative one point five"
    )]
    #[test_case(
        Dec128::MIN // -2^127 / 10^18 = -170,141,183,460,469,231,731.687303715884105728
        => (
            -Int128::new(170_141_183_460_469_231_731), // rounded towards zero
            -Int128::new(170_141_183_460_469_231_732), // rounded down by 1
            -Int128::new(170_141_183_460_469_231_731), // rounded up by 1
        );
        "signed negative max"
    )]
    fn into_int<U, const S: u32>(dec: Dec<U, S>) -> (Int<U>, Int<U>, Int<U>)
    where
        Dec<U, S>: FixedPoint<U> + Copy,
        Int<U>: Number + NumberConst + Sign + IsZero,
    {
        (dec.into_int(), dec.into_int_floor(), dec.into_int_ceil())
    }
}
