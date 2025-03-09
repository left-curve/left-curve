use {
    crate::{
        Dec128, Dec256, Int, Int128, Int256, NumberConst, Udec128, Udec128_6, Udec128_9, Udec256,
        Uint128, Uint256,
    },
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

impl FixedPoint<u128> for Udec128_6 {
    const PRECISION: Uint128 = Uint128::new(10_u128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Uint128::ONE);
}

impl FixedPoint<u128> for Udec128_9 {
    const PRECISION: Uint128 = Uint128::new(10_u128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Uint128::ONE);
}

impl FixedPoint<u128> for Udec128 {
    const PRECISION: Uint128 = Uint128::new(10_u128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Uint128::ONE);
}

impl FixedPoint<U256> for Udec256 {
    const PRECISION: Uint256 = Uint256::new_from_u128(10_u128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Uint256::ONE);
}

impl FixedPoint<i128> for Dec128 {
    const PRECISION: Int128 = Int128::new(10_i128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Int128::ONE);
}

impl FixedPoint<I256> for Dec256 {
    const PRECISION: Int256 = Int256::new_from_i128(10_i128.pow(Self::DECIMAL_PLACES));
    const TICK: Self = Self::raw(Int256::ONE);
}

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
