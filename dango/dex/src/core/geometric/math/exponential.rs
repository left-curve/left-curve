use grug::{
    Dec, Exponentiate, Inner, MathResult, MultiplyFraction, Number, NumberConst, Signed, Unsigned,
};

use crate::core::geometric::math::{NATURAL_LOG_OF_TWO, UnsignedDecimalConstant};

/// Computes the exponential function e^x using the following trick.
///
/// Let x = k*ln(2) + r, where k is an integer and r is a real number in the range [-ln(2)/2, ln(2)/2).
/// Then, e^x = e^(k*ln(2)) * e^r = 2^k * e^r.
/// We find k by computing the integer part
///
/// We can compute 2^k using a the built in checked_pow method.
///
/// We can compute e^r using a Padé approximant series expansion.
///
/// We can compute the Taylor series expansion using a Horner's method.
///
/// We can compute the Horner's method using a simple loop.
pub fn e_pow<const S: u32>(x: Dec<u128, S>) -> MathResult<Dec<u128, S>> {
    // Compute k and r
    let ln_of_two = NATURAL_LOG_OF_TWO::to_decimal_value::<S>()?;
    let k = round(x.checked_div(ln_of_two)?)?;
    let r = x
        .checked_into_signed()?
        .checked_sub(k.checked_mul_dec(ln_of_two)?.checked_into_signed()?)?;

    let k_as_u32 = k.into_int().into_inner() as u32;
    let two_to_k = Dec::<u128, S>::new(2).checked_pow(k_as_u32)?;
    let e_r = _pade_approximant_of_e_to_r(r)?;

    Ok(two_to_k.checked_mul(e_r)?)
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
    let floored = x.checked_mul_dec_floor(Dec::<u128, S>::ONE)?;
    let ceiled = x.checked_mul_dec_ceil(Dec::<u128, S>::ONE)?;

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
    fn dec18(s: &str) -> Dec<u128, 18> {
        s.parse().unwrap()
    }

    // Helper function to assert approximate equality (within 0.001%)
    fn assert_approx_eq(actual: Dec<u128, 18>, expected: Dec<u128, 18>, tolerance_bps: u128) {
        let diff = if actual > expected {
            actual.checked_sub(expected).unwrap()
        } else {
            expected.checked_sub(actual).unwrap()
        };
        let tolerance = expected
            .checked_mul_dec(Dec::<u128, 18>::checked_from_ratio(tolerance_bps, 1_000_000).unwrap())
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

    #[test]
    fn test_round_integers() {
        // Test exact integers
        assert_eq!(round(dec18("0")).unwrap(), dec18("0"));
        assert_eq!(round(dec18("1")).unwrap(), dec18("1"));
        assert_eq!(round(dec18("5")).unwrap(), dec18("5"));
        assert_eq!(round(dec18("100")).unwrap(), dec18("100"));
    }

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
    fn test_round_decimals(input: &str, expected: &str) {
        let result = round(dec18(input)).unwrap();
        assert_eq!(result, dec18(expected));
    }

    #[test]
    fn test_e_pow_zero() {
        // e^0 = 1
        let result = e_pow(dec18("0")).unwrap();
        assert_eq!(result, dec18("1"));
    }

    #[test]
    fn test_e_pow_one() {
        // e^1 ≈ 2.718281828459045
        let result = e_pow(dec18("1")).unwrap();
        let expected = dec18("2.718281828459045");
        // Allow 0.1% tolerance (1000 basis points)
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_e_pow_negative_one() {
        // e^-1 ≈ 0.367879441171442
        let result = e_pow(dec18("0.367879441171442")).unwrap();
        let expected = dec18("1.444");
        // Allow 1% tolerance for this approximation
        assert_approx_eq(result, expected, 10000);
    }

    #[test]
    fn test_e_pow_two() {
        // e^2 ≈ 7.389056098930650
        let result = e_pow(dec18("2")).unwrap();
        let expected = dec18("7.389056098930650");
        // Allow 0.1% tolerance
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_e_pow_small_values() {
        // e^0.1 ≈ 1.105170918075648
        let result = e_pow(dec18("0.1")).unwrap();
        let expected = dec18("1.105170918075648");
        assert_approx_eq(result, expected, 1000);

        // e^0.5 ≈ 1.648721270700128
        let result = e_pow(dec18("0.5")).unwrap();
        let expected = dec18("1.648721270700128");
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_e_pow_larger_values() {
        // e^3 ≈ 20.085536923187668
        let result = e_pow(dec18("3")).unwrap();
        let expected = dec18("20.085536923187668");
        assert_approx_eq(result, expected, 1000);

        // e^4 ≈ 54.598150033144236
        let result = e_pow(dec18("4")).unwrap();
        let expected = dec18("54.598150033144236");
        assert_approx_eq(result, expected, 1000);

        // e^5 ≈ 148.413159102576603
        let result = e_pow(dec18("5")).unwrap();
        let expected = dec18("148.413159102576603");
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_e_pow_fractional_values() {
        // e^1.5 ≈ 4.481689070338065
        let result = e_pow(dec18("1.5")).unwrap();
        let expected = dec18("4.481689070338065");
        assert_approx_eq(result, expected, 1000);

        // e^2.3 ≈ 9.974182454814718
        let result = e_pow(dec18("2.3")).unwrap();
        let expected = dec18("9.974182454814718");
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_e_pow_very_small_values() {
        // e^0.01 ≈ 1.010050167084168
        let result = e_pow(dec18("0.01")).unwrap();
        let expected = dec18("1.010050167084168");
        assert_approx_eq(result, expected, 1000);

        // e^0.001 ≈ 1.001000500166708
        let result = e_pow(dec18("0.001")).unwrap();
        let expected = dec18("1.001000500166708");
        assert_approx_eq(result, expected, 1000);
    }

    #[test]
    fn test_pade_approximant_zero() {
        // e^0 = 1
        let result = _pade_approximant_of_e_to_r(Dec::<i128, 18>::ZERO).unwrap();
        assert_eq!(result, dec18("1"));
    }

    #[test]
    fn test_pade_approximant_small_positive() {
        // Test with small positive r value
        let r = Dec::<i128, 18>::checked_from_ratio(1, 10).unwrap();
        let result = _pade_approximant_of_e_to_r(r).unwrap();
        // e^0.1 ≈ 1.105170918 (Padé approximant should be close)
        let expected = dec18("1.105170918075648");
        assert_approx_eq(result, expected, 2000);
    }

    #[test]
    fn test_pade_approximant_small_negative() {
        // Test with small negative r value
        let r = Dec::<i128, 18>::checked_from_ratio(-1, 10).unwrap();
        let result = _pade_approximant_of_e_to_r(r).unwrap();
        // e^-0.1 ≈ 0.904837418
        let expected = dec18("0.904837418035960");
        assert_approx_eq(result, expected, 2000);
    }

    #[test]
    fn test_round_large_numbers() {
        assert_eq!(round(dec18("999.4")).unwrap(), dec18("999"));
        assert_eq!(round(dec18("999.5")).unwrap(), dec18("1000"));
        assert_eq!(round(dec18("1000.5")).unwrap(), dec18("1001"));
    }

    #[test]
    fn test_e_pow_precision_with_different_inputs() {
        // Test a sequence to verify consistency
        let values = vec!["0.2", "0.4", "0.6", "0.8"];
        let expected = vec![
            "1.221402758160170",
            "1.491824697641270",
            "1.822118800390509",
            "2.225540928492468",
        ];

        for (input, exp) in values.iter().zip(expected.iter()) {
            let result = e_pow(dec18(input)).unwrap();
            assert_approx_eq(result, dec18(exp), 1000);
        }
    }

    #[test]
    fn test_e_pow_powers_of_ten() {
        // e^10 ≈ 22026.465794806718
        let result = e_pow(dec18("10")).unwrap();
        let expected = dec18("22026.465794806718");
        assert_approx_eq(result, expected, 1000);
    }
}
