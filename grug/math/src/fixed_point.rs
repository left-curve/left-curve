use crate::Int;

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
