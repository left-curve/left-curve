use {
    crate::{Dec, Int, NumberConst},
    bnum::types::{I256, U256},
};

/// Describes a [fixed-point decimal](https://en.wikipedia.org/wiki/Fixed-point_arithmetic)
/// number.
pub trait FixedPoint<U> {
    /// Ratio between the inner integer value and the decimal value it represents.
    ///
    /// This should always be `10 ^ DECIMAL_PLACES`.
    const PRECISION: Int<U>;

    /// The smallest incremental value that can be represented.
    ///
    /// For `Dec<U>`, this is typically `Dec::raw(Int::<U>::ONE)`.
    const TICK: Self;
}

// ------------------------------------ dec ------------------------------------

impl<const S: u32> FixedPoint<u128> for Dec<u128, S> {
    const PRECISION: Int<u128> = Int::<u128>::new(10u128.pow(S));
    const TICK: Self = Self::raw(Int::<u128>::ONE);
}

impl<const S: u32> FixedPoint<i128> for Dec<i128, S> {
    const PRECISION: Int<i128> = Int::<i128>::new(10i128.pow(S));
    const TICK: Self = Self::raw(Int::<i128>::ONE);
}

impl<const S: u32> FixedPoint<U256> for Dec<U256, S> {
    const PRECISION: Int<U256> = Int::<U256>::new_from_u128(10u128.pow(S));
    const TICK: Self = Self::raw(Int::<U256>::ONE);
}

impl<const S: u32> FixedPoint<I256> for Dec<I256, S> {
    const PRECISION: Int<I256> = Int::<I256>::new_from_i128(10i128.pow(S));
    const TICK: Self = Self::raw(Int::<I256>::ONE);
}

// Trait auto-impl for all decimals via `generate_decimals` macro.

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{FixedPoint, Int, dec_test},
        bnum::types::{I256, U256},
        std::fmt::Debug,
    };

    dec_test!( fixed_point
        inputs = {
            udec128 = {
                passing: [
                    1_000_000_000_000_000_000_u128
                ]
            }
            udec256 = {
                passing: [
                     U256::from(1_000_000_000_000_000_000_u128)
                ]
            }
            dec128 = {
                passing: [
                     1_000_000_000_000_000_000_i128
                ]
            }
            dec256 = {
                passing: [
                    I256::from(1_000_000_000_000_000_000_i128)
                ]
            }
        }
        method = |_0d, passing| {
            for  precision in passing {
                fn t<U, FP: FixedPoint<U>>(_: FP,  precision: Int<U>)
                where Int<U>: PartialEq + Debug {
                    assert_eq!(FP::PRECISION, precision);
                }
                t(_0d,  Int::new(precision));
            }
        }
    );
}
