use grug::{
    Dec, Dec128, Inner, Int128, MathError, MultiplyFraction, Number, NumberConst, PrevNumber,
    Unsigned,
};

use crate::core::geometric::logarithm::{
    constants::*,
    lut::{LOG2_LUT_ONE_TO_TWO, TABLE_SIZE},
};

/// Converts a u64 to a Dec128 with 18 decimal places
///
/// ```rust
/// let value = 1442695040888963407;
/// let dec = _u64_to_dec128(value).unwrap();
/// assert_eq!(dec, Dec128::from_str("1.442695040888963407").unwrap());
/// ```
fn _u64_to_dec128<const S: u32>(value: u64) -> Result<Dec<i128, S>, MathError> {
    Dec::<i128, S>::checked_from_atomics(Int128::new(value as i128), 18)
}

/// Computes an approximation of the two-logarithm of a number in the range [1, 2)
/// using Hermite interpolation of a lookup table
///
/// ## Inputs
/// * `x` - The value to compute the logarithm of (must be in the range [1, 2)
///
/// ## Outputs
/// * The logarithm of x
fn _log2_interpolated_one_to_two<const S: u32>(x: Dec<i128, S>) -> anyhow::Result<Dec<i128, S>> {
    if x < Dec::<i128, S>::ONE || x >= Dec::<i128, S>::new_percent(200) {
        anyhow::bail!("input must be in the range [1, 2). got {}", x);
    }

    let table_size = Int128::new(TABLE_SIZE as i128);

    // Compute index in LUT
    let pre_index = table_size.checked_mul_dec(x - Dec::<i128, S>::ONE)?;
    let index = pre_index.checked_into_prev()?.into_inner() as usize;
    let index = index.min((TABLE_SIZE - 2) as usize); // Avoid out-of-bounds access

    // Get x coordinates
    let x1 = Dec::<i128, S>::ONE + Dec::<i128, S>::checked_from_ratio(index as i128, table_size)?;
    let x2 = x1 + Dec::<i128, S>::checked_from_ratio(Int128::ONE, table_size)?;

    // Get function values
    let y1 = _u64_to_dec128::<S>(LOG2_LUT_ONE_TO_TWO[index])?;
    let y2 = _u64_to_dec128::<S>(LOG2_LUT_ONE_TO_TWO[index + 1])?;

    // Compute derivatives at points
    // For log2(x), the derivative is 1/(x*ln(2))
    let one_over_ln2 = _u64_to_dec128::<S>(ONE_OVER_NATURAL_LOG_OF_TWO)?;
    let m1 = one_over_ln2.checked_div(x1)?;
    let m2 = one_over_ln2.checked_div(x2)?;

    // Compute Hermite basis functions
    let t = (x - x1).checked_div(x2.checked_sub(x1)?)?;
    let t2 = t.checked_mul(t)?;
    let t3 = t2.checked_mul(t)?;

    // Compute the basis functions at t
    // h00 = 2t³ - 3t² + 1
    // h10 = t³ - 2t² + t
    // h01 = -2t³ + 3t²
    // h11 = t³ - t²
    let h00 = Dec::<i128, S>::new(2)
        .checked_mul(t3)?
        .checked_sub(Dec::<i128, S>::new(3).checked_mul(t2)?)?
        .checked_add(Dec::<i128, S>::ONE)?;
    let h10 = t3
        .checked_sub(Dec::<i128, S>::new(2).checked_mul(t2)?)?
        .checked_add(t)?;
    let h01 = Dec::<i128, S>::new(-2)
        .checked_mul(t3)?
        .checked_add(Dec::<i128, S>::new(3).checked_mul(t2)?)?;
    let h11 = t3.checked_sub(t2)?;

    // Compute final interpolated value
    // y(x) = h00 * y1 + h10 * (x2 - x1) * m1 + h01 * y2 + h11 * (x2 - x1) * m2
    let dx = x2.checked_sub(x1)?;
    let result = h00
        .checked_mul(y1)?
        .checked_add(h10.checked_mul(dx)?.checked_mul(m1)?)?
        .checked_add(h01.checked_mul(y2)?)?
        .checked_add(h11.checked_mul(dx)?.checked_mul(m2)?)?;

    Ok(result)
}

pub fn ln_interpolated_one_to_two<const S: u32>(x: Dec<i128, S>) -> anyhow::Result<Dec<i128, S>> {
    if x < Dec::<i128, S>::ONE || x >= Dec::<i128, S>::new_percent(200) {
        anyhow::bail!("input must be in the range [1, 2). got {}", x);
    }

    // Use logarithm conversion formula: ln(x) = log2(x) * ln(2)
    // Since ONE_OVER_NATURAL_LOG_OF_TWO = 1/ln(2), we divide by it to get ln(2)
    let log2_x = _log2_interpolated_one_to_two(x)?;
    let ln_of_two = NATURAL_LOG_OF_TWO::to_decimal_value::<S>()?.checked_into_signed()?;

    Ok(log2_x.checked_mul(ln_of_two)?)
}

/// Computes the base-2 logarithm of an i128 value
///
/// This implementation splits the calculation into integer and fractional parts:
/// For a number x = 2^(i + f) where i is integer and 0 ≤ f < 1
/// log2(x) = i + f
///
/// The integer part i is computed using checked_ilog2
/// The fractional part f is computed using a lookup table and Hermite interpolation
///
/// ## Inputs
/// * `x` - The i128 value to compute the logarithm of (must be positive)
///
/// ## Outputs
/// * `Ok(Dec128)` - The base-2 logarithm of x
/// * `Err` - If x is not positive or numerical error occurs
pub fn log2_i128(x: i128) -> anyhow::Result<Dec128> {
    // Ensure input is positive
    anyhow::ensure!(x > 0, "Logarithm is only defined for positive numbers");

    // Special case for x = 1
    if x == 1 {
        return Ok(Dec128::ZERO);
    }

    // Get the integer part of log2(x)
    let i = x
        .checked_ilog2()
        .ok_or_else(|| anyhow::anyhow!("ilog2 failed"))?;
    let i_dec = Dec128::new(i as i128);

    // If x is a perfect power of 2, we're done
    if x == (2i128 << (i - 1)) {
        return Ok(i_dec);
    }

    // Calculate x/2^i which will be in [1,2)
    let two_to_i = 2i128
        .checked_pow(i)
        .ok_or_else(|| anyhow::anyhow!("pow failed"))?;
    let normalized = Dec128::checked_from_ratio(Int128::new(x), Int128::new(two_to_i))?;

    // Now we need to find f where 2^f = normalized
    let f = _log2_interpolated_one_to_two(normalized)?;

    // Combine integer and fractional parts
    Ok(i_dec.checked_add(f)?)
}

#[cfg(test)]
mod tests {
    use {super::*, std::str::FromStr, test_case::test_case};

    #[test_case(
        1i128
        => Dec128::ZERO
        ; "log2_of_1"
    )]
    #[test_case(
        3i128
        => Dec128::from_str("1.584962500721156181").unwrap()
        ; "log2_of_3"
    )]
    #[test_case(
        10i128
        => Dec128::from_str("3.321928094887362348").unwrap()
        ; "log2_of_10"
    )]
    #[test_case(
        32i128
        => Dec128::from_str("5").unwrap()
        ; "log2_of_32"
    )]
    #[test_case(
        33i128
        => Dec128::from_str("5.044394119358453438").unwrap()
        ; "log2_of_33"
    )]
    #[test_case(
        100i128
        => Dec128::from_str("6.643856189774724696").unwrap()
        ; "log2_of_100"
    )]
    #[test_case(
        1000000000000i128
        => Dec128::from_str("39.863137138648348173").unwrap()
        ; "log2_of_10_to_the_12th"
    )]
    #[test_case(
        1000000000000000i128
        => Dec128::from_str("49.828921423310435217").unwrap()
        ; "log2_of_10_to_the_15th"
    )]
    #[test_case(
        1000000000000000000i128
        => Dec128::from_str("59.794705707972522261").unwrap()
        ; "log2_of_10_to_the_18th"
    )]
    #[test_case(
        100000000000000000000i128
        => Dec128::from_str("66.438561897747246960").unwrap()
        ; "log2_of_10_to_the_20th"
    )]
    fn test_log2_i128(x: i128) -> Dec128 {
        log2_i128(x).unwrap()
    }

    // Tests for ln_interpolated_one_to_two with correct implementation: ln(x) = log2(x) / log2(e)
    #[test_case(
        Dec128::ONE
        => Dec128::ZERO
        ; "ln_of_1"
    )]
    #[test_case(
        Dec128::from_str("1.1").unwrap()
        => Dec128::from_str("0.095310179804324941").unwrap()  // ln(1.1), within interpolation precision
        ; "ln_of_1_1"
    )]
    #[test_case(
        Dec128::from_str("1.25").unwrap()
        => Dec128::from_str("0.223143551314209755").unwrap()  // ln(1.25)
        ; "ln_of_1_25"
    )]
    #[test_case(
        Dec128::from_str("1.5").unwrap()
        => Dec128::from_str("0.405465108108164381").unwrap()  // ln(1.5)
        ; "ln_of_1_5"
    )]
    #[test_case(
        Dec128::from_str("1.75").unwrap()
        => Dec128::from_str("0.559615787935422686").unwrap()  // ln(1.75)
        ; "ln_of_1_75"
    )]
    #[test_case(
        Dec128::from_str("1.9").unwrap()
        => Dec128::raw(Int128::new(641853886172394903))  // ln(1.9), within interpolation precision
        ; "ln_1_9_corrected"
    )]
    #[test_case(
        Dec128::from_str("1.99").unwrap()
        => Dec128::raw(Int128::new(688134858305229874))  // ln(1.99), within interpolation precision
        ; "ln_1_99_corrected"
    )]
    #[test_case(
        Dec128::from_str("1.999").unwrap()
        => Dec128::raw(Int128::new(693647198106155077))  // ln(1.999), within interpolation precision
        ; "ln_1_999_corrected"
    )]
    fn test_ln_interpolated_one_to_two(x: Dec128) -> Dec128 {
        ln_interpolated_one_to_two::<24>(x.convert_precision::<24>().unwrap())
            .unwrap()
            .convert_precision()
            .unwrap()
    }
}
