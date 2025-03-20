use crate::{Dec, FixedPoint, Int, MathResult, MultiplyRatio};

/// Describes a number that can be expressed as the quotient of two integers.
pub trait Fraction<U>: Sized {
    fn numerator(&self) -> Int<U>;

    fn denominator() -> Int<U>;

    fn checked_inv(&self) -> MathResult<Self>;

    #[inline]
    fn checked_inv_assign(&mut self) -> MathResult<()> {
        *self = self.checked_inv()?;
        Ok(())
    }
}

impl<U, const S: u32> Fraction<U> for Dec<U, S>
where
    Self: FixedPoint<U>,
    U: Copy,
    Int<U>: MultiplyRatio,
{
    fn numerator(&self) -> Int<U> {
        self.0
    }

    fn denominator() -> Int<U> {
        Self::PRECISION
    }

    fn checked_inv(&self) -> MathResult<Self> {
        Self::checked_from_ratio(Self::PRECISION, self.0)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{
            Dec, Dec128, Dec256, FixedPoint, Fraction, Int, Int128, Int256, MathError, NumberConst,
            Udec128, Udec256, Uint128, Uint256, dec_test, test_utils::dt,
        },
        bnum::types::{I256, U256},
        std::fmt::Debug,
    };

    dec_test!( numerator
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::ZERO, Uint128::ZERO),
                    (Udec128::TEN, Uint128::TEN * Udec128::PRECISION),
                    (Udec128::MAX, Uint128::MAX),
                ]
            }
            udec256 = {
                passing: [
                    (Udec256::ZERO, Uint256::ZERO),
                    (Udec256::TEN, Uint256::TEN * Udec256::PRECISION),
                    (Udec256::MAX, Uint256::MAX),
                ]
            }
            dec128 = {
                passing: [
                    (Dec128::ZERO, Int128::ZERO),
                    (Dec128::TEN, Int128::TEN * Dec128::PRECISION),
                    (Dec128::MAX, Int128::MAX),
                    (-Dec128::TEN, -Int128::TEN * Dec128::PRECISION),
                    (Dec128::MIN, Int128::MIN),
                ]
            }
            dec256 = {
                passing: [
                    (Dec256::ZERO, Int256::ZERO),
                    (Dec256::TEN, Int256::TEN * Dec256::PRECISION),
                    (Dec256::MAX, Int256::MAX),
                    (-Dec256::TEN, -Int256::TEN * Dec256::PRECISION),
                    (Dec256::MIN, Int256::MIN),
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing| {
            for (base, numerator) in passing {
                dt(_0d, base);
                assert_eq!(base.numerator(), numerator);
            }
        }
    );

    dec_test!( denominator
        inputs = {
            udec128 = {
                passing: [
                    1_000_000_000_000_000_000_u128,
                ]
            }
            udec256 = {
                passing: [
                    U256::from(1_000_000_000_000_000_000_u128),
                ]
            }
            dec128 = {
                passing: [
                    1_000_000_000_000_000_000_i128,
                ]
            }
            dec256 = {
                passing: [
                    I256::from(1_000_000_000_000_000_000_i128),
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing| {
            for denominator in passing {

                fn t<U, FP: Fraction<U>>(_: FP,  denominator: Int<U>)
                where Int<U>: PartialEq + Debug {
                    assert_eq!(FP::denominator(), denominator);
                }

                t(_0d, Int::new(denominator));
            }
        }
    );

    dec_test!( checked_inv
        inputs = {
            udec128 = {
                passing: [
                    (Udec128::new_percent(20), Udec128::new(5)),
                    (Udec128::TEN, Udec128::new_percent(10)),
                    (Udec128::new(100), Udec128::new_percent(1)),
                    (Udec128::MAX, Udec128::ZERO),
                ]
            }
            udec256 = {
                passing: [
                    (Udec256::new_percent(20), Udec256::new(5)),
                    (Udec256::TEN, Udec256::new_percent(10)),
                    (Udec256::new(100), Udec256::new_percent(1)),
                    (Udec256::MAX, Udec256::ZERO),
                ]
            }
            dec128 = {
                passing: [
                    (Dec128::new_percent(20), Dec128::new(5)),
                    (Dec128::TEN, Dec128::new_percent(10)),
                    (Dec128::new(100), Dec128::new_percent(1)),
                    (Dec128::MAX, Dec128::ZERO),
                    (Dec128::new_percent(-20), Dec128::new(-5)),
                    (-Dec128::TEN, Dec128::new_percent(-10)),
                    (Dec128::new(-100), Dec128::new_percent(-1)),
                    (Dec128::MIN, Dec128::ZERO),
                ]
            }
            dec256 = {
                passing: [
                    (Dec256::new_percent(20), Dec256::new(5)),
                    (Dec256::TEN, Dec256::new_percent(10)),
                    (Dec256::new(100), Dec256::new_percent(1)),
                    (Dec256::MAX, Dec256::ZERO),
                    (Dec256::new_percent(-20), Dec256::new(-5)),
                    (-Dec256::TEN, Dec256::new_percent(-10)),
                    (Dec256::new(-100), Dec256::new_percent(-1)),
                    (Dec256::MIN, Dec256::ZERO),
                ]
            }
        }
        method = |_0d: Dec<_, 18>, passing| {
            for (base, inv) in passing {
                dt(_0d, base);
                assert_eq!(base.checked_inv().unwrap(), inv);
            }

            assert!(matches!(_0d.checked_inv(), Err(MathError::DivisionByZero { .. })));
        }
    );
}
