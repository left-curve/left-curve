use grug::{
    Dec, Decimal, Exponentiate, Inner, MathResult, MultiplyFraction, Number, NumberConst, Signed,
    Unsigned,
};

use crate::core::geometric::math::{NaturalLogOfTwo, UnsignedDecimalConstant};

/// Computes the exponential function e^x using the following trick.
///
/// Let x = k*ln(2) + r, where k is an integer and r is a real number in the range [-ln(2)/2, ln(2)/2).
/// Then, e^x = e^(k*ln(2)) * e^r = 2^k * e^r.
/// We find k by computing the integer part
///
/// We can compute 2^k using a the built in checked_pow method.
///
/// We can compute e^r using a Padé approximant series expansion.
pub fn e_pow<const S: u32>(x: Dec<u128, S>) -> MathResult<Dec<u128, S>> {
    // Compute k and r
    let ln_of_two = NaturalLogOfTwo::to_decimal_value::<S>()?;
    let k = round(x.checked_div(ln_of_two)?)?;
    let r = x
        .checked_into_signed()?
        .checked_sub(k.checked_mul_dec(ln_of_two)?.checked_into_signed()?)?;

    let k_as_u32 = k.into_int().into_inner() as u32;
    let two_to_k = Dec::<u128, S>::new(2).checked_pow(k_as_u32)?;
    let e_r = _pade_approximant_of_e_to_r(r)?;

    two_to_k.checked_mul(e_r)
}

/// Computes the Padé (2,2) approximant of e^r, where -ln(2)/2 <= r < ln(2)/2, using the following
/// formula:
///
/// e^r ≈ (1 + r/2 + r^2/12) / (1 - r/2 + r^2/12)
///
/// See https://www.wolframalpha.com/input?i=PadeApproximant%5Be%5Ex%2C+%7Bx%2C0%2C%7B2%2C2%7D%7D%5D
fn _pade_approximant_of_e_to_r<const S: u32>(r: Dec<i128, S>) -> MathResult<Dec<u128, S>> {
    let a0 = Dec::<i128, S>::ONE;
    let a1 = r.checked_div(Dec::<i128, S>::new(2))?;
    let a2 = r.checked_mul(r)?.checked_div(Dec::<i128, S>::new(12))?;

    let b0 = Dec::<i128, S>::ONE;
    let b1 = r.checked_div(Dec::<i128, S>::new(-2))?;
    let b2 = r.checked_mul(r)?.checked_div(Dec::<i128, S>::new(12))?;

    let numerator = a0.checked_add(a1)?.checked_add(a2)?;
    let denominator = b0.checked_add(b1)?.checked_add(b2)?;
    let result = numerator.checked_div(denominator)?;

    result.checked_into_unsigned()
}

/// Rounds a decimal number to the nearest integer.
pub fn round<const S: u32>(x: Dec<u128, S>) -> MathResult<Dec<u128, S>> {
    let floored = x.checked_floor()?;
    let ceiled = x.checked_ceil()?;

    if x.checked_sub(floored)? < Dec::<u128, S>::checked_from_ratio(1, 2)? {
        Ok(floored)
    } else {
        Ok(ceiled)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    // Helper function to create a Decimal with 18 decimal places
    fn udec18(s: &str) -> Dec<u128, 18> {
        s.parse().unwrap()
    }

    fn dec18(s: &str) -> Dec<i128, 18> {
        s.parse().unwrap()
    }

    // Helper function to assert approximate equality (within 0.001%)
    fn assert_approx_eq(actual: Dec<u128, 18>, expected: Dec<u128, 18>, tolerance_ppm: u128) {
        let diff = if actual > expected {
            actual.checked_sub(expected).unwrap()
        } else {
            expected.checked_sub(actual).unwrap()
        };
        let tolerance = expected
            .checked_mul_dec(Dec::<u128, 18>::checked_from_ratio(tolerance_ppm, 1_000_000).unwrap())
            .unwrap();
        assert!(
            diff <= tolerance,
            "Values differ by more than tolerance: actual={}, expected={}, diff={}, tolerance={}",
            actual,
            expected,
            diff,
            tolerance
        );
    }

    #[test_case("0", "0" ; "rounds 0 to 0")]
    #[test_case("1", "1" ; "rounds 1 to 1")]
    #[test_case("5", "5" ; "rounds 5 to 5")]
    #[test_case("100", "100" ; "rounds 100 to 100")]
    #[test_case("0.1", "0" ; "rounds 0.1 down")]
    #[test_case("0.4", "0" ; "rounds 0.4 down")]
    #[test_case("0.49", "0" ; "rounds 0.49 down")]
    #[test_case("0.5", "1" ; "rounds 0.5 up")]
    #[test_case("0.51", "1" ; "rounds 0.51 up")]
    #[test_case("0.9", "1" ; "rounds 0.9 up")]
    #[test_case("1.4", "1" ; "rounds 1.4 down")]
    #[test_case("1.5", "2" ; "rounds 1.5 up")]
    #[test_case("1.6", "2" ; "rounds 1.6 up")]
    #[test_case("2.5", "3" ; "rounds 2.5 up")]
    #[test_case("10.499", "10" ; "rounds 10.499 down")]
    #[test_case("10.500", "11" ; "rounds 10.5 up")]
    #[test_case("999.4", "999" ; "rounds 999.4 down")]
    #[test_case("999.5", "1000" ; "rounds 999.5 up")]
    #[test_case("1000.5", "1001" ; "rounds 1000.5 up")]
    fn test_round_decimals(input: &str, expected: &str) {
        let result = round(udec18(input)).unwrap();
        assert_eq!(result, udec18(expected));
    }

    #[test_case("0", "1", 1000 ; "e^0 = 1")]
    #[test_case("0.2", "1.221402758160170", 1000 ; "e^0.2 = 1.221402758160170")]
    #[test_case("0.4", "1.491824697641270", 1000 ; "e^0.4 = 1.491824697641270")]
    #[test_case("0.6", "1.822118800390509", 1000 ; "e^0.6 = 1.822118800390509")]
    #[test_case("0.8", "2.225540928492468", 1000 ; "e^0.8 = 2.225540928492468")]
    #[test_case("1", "2.718281828459045", 1000 ; "e^1 = 2.718281828459045")]
    #[test_case("0.1", "1.105170918075648", 1000 ; "e^0.1 = 1.105170918075648")]
    #[test_case("0.5", "1.648721270700128", 1000 ; "e^0.5 = 1.648721270700128")]
    #[test_case("2.0", "7.389056098930650", 1000 ; "e^2.0 = 7.389056098930650")]
    #[test_case("3", "20.085536923187668", 1000 ; "e^3 = 20.085536923187668")]
    #[test_case("4", "54.598150033144236", 1000 ; "e^4 = 54.598150033144236")]
    #[test_case("5", "148.413159102576603", 1000 ; "e^5 = 148.413159102576603")]
    #[test_case("10", "22026.465794806718", 1000 ; "e^10 = 22026.465794806718")]
    #[test_case("1.5", "4.481689070338065", 1000 ; "e^1.5 = 4.481689070338065")]
    #[test_case("2.3", "9.974182454814718", 1000 ; "e^2.3 = 9.974182454814718")]
    #[test_case("0.01", "1.010050167084168", 1000 ; "e^0.01 = 1.010050167084168")]
    #[test_case("0.001", "1.001000500166708", 1000 ; "e^0.001 = 1.001000500166708")]
    fn test_e_pow(input: &str, expected: &str, tolerance: u128) {
        // e^0 = 1
        let result = e_pow(udec18(input)).unwrap();
        assert_approx_eq(result, udec18(expected), tolerance);
    }

    #[test_case("0", "1", 0 ; "e^0 = 1")]
    #[test_case("-0.1", "0.904837418035960", 1 ; "e^-0.1 = 0.904837418035960")]
    #[test_case("0.1", "1.105170918075648", 1 ; "e^0.1 = 1.105170918075648")]
    #[test_case("-0.34", "0.7117703228", 10 ; "e^-0.34 = 0.710542735760100")]
    #[test_case("0.34", "1.4049475906", 10 ; "e^0.34 = 1.403119252053656")]
    fn test_pade_approximant(input: &str, expected: &str, tolerance: u128) {
        let result = _pade_approximant_of_e_to_r(dec18(input)).unwrap();
        assert_approx_eq(result, udec18(expected), tolerance);
    }

    #[test]
    fn test_e_pow_small_exponent_24_decimals() {
        // Regression test for bug where e_pow returned 1.0 for small exponents with 24 decimal places
        // This value comes from the volatility estimator: ln(2) * dt / half_life = 0.693... * 1000 / 5000
        use std::str::FromStr;

        let x = Dec::<u128, 24>::from_str("0.138629436111989061883446").unwrap();
        let result = e_pow(x).unwrap();

        // e^0.1386... ≈ 1.1487
        let expected = Dec::<u128, 24>::from_str("1.148698354997035").unwrap();

        // Check result is close to expected (within 0.1% tolerance)
        let diff = if result > expected {
            result.checked_sub(expected).unwrap()
        } else {
            expected.checked_sub(result).unwrap()
        };
        let tolerance = expected
            .checked_mul_dec(Dec::<u128, 24>::checked_from_ratio(1000, 1_000_000).unwrap())
            .unwrap();

        assert!(
            diff <= tolerance,
            "e_pow returned {}, expected {}, diff {} exceeds tolerance {}",
            result,
            expected,
            diff,
            tolerance
        );
    }
}
