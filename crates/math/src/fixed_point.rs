use bnum::types::U256;

use crate::{Udec128, Udec256, Uint, Uint128, Uint256};

pub trait FixedPoint<U> {
    /// Ratio between the inner integer value and the decimal value it
    /// represents.
    const DECIMAL_FRACTION: Uint<U>;

    /// Number of decimal digits to be interpreted as decimal places.
    const DECIMAL_PLACES: u32;
}

macro_rules! impl_fixed_point {
    ($t:ty => $u:ty, $constructor:expr, $dp:expr) => {
        impl FixedPoint<$u> for $t {
            const DECIMAL_FRACTION: Uint<$u> = $constructor(10_u128.pow($dp));
            const DECIMAL_PLACES: u32 = $dp;
        }
    };
}

impl_fixed_point!(Udec128 => u128, Uint128::new, 18);
impl_fixed_point!(Udec256 => U256, Uint256::new_from_u128, 18);
