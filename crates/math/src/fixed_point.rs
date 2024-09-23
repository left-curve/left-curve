use bnum::types::{I256, U256};

use crate::{Dec128, Dec256, Int128, Int256, Udec128, Udec256, Uint, Uint128, Uint256};

pub trait FixedPoint<U> {
    /// Ratio between the inner integer value and the decimal value it
    /// represents.
    const DECIMAL_FRACTION: Uint<U>;

    /// Number of decimal digits to be interpreted as decimal places.
    const DECIMAL_PLACES: u32;
}

macro_rules! impl_fixed_point {
    ($t:ty => $u:ty, $base_constructor:ty, $constructor:expr, $dp:expr) => {
        paste::paste! {
            impl FixedPoint<$u> for $t {
                const DECIMAL_FRACTION: Uint<$u> = $constructor([<10_$base_constructor>].pow($dp));
                const DECIMAL_PLACES: u32 = $dp;
            }
        }
    };
    (
        type = Unsigned,for =
        $t:ty =>
        $u:ty,inner_constructor =
        $constructor:expr,decimal_places =
        $dp:expr
    ) => {
        impl_fixed_point! { $t => $u, u128, $constructor, $dp }
    };
    (
        type = Signed,for =
        $t:ty =>
        $u:ty,inner_constructor =
        $constructor:expr,decimal_places =
        $dp:expr
    ) => {
        impl_fixed_point! { $t => $u, i128, $constructor, $dp }
    };
}

impl_fixed_point! {
    type              = Unsigned,
    for               = Udec128 => u128,
    inner_constructor = Uint128::new,
    decimal_places    = 18
}

impl_fixed_point! {
    type              = Unsigned,
    for               = Udec256 => U256,
    inner_constructor = Uint256::new_from_u128,
    decimal_places    = 18
}

impl_fixed_point! {
    type              = Signed,
    for               = Dec128 => i128,
    inner_constructor = Int128::new,
    decimal_places    = 18
}

impl_fixed_point! {
    type              = Signed,
    for               = Dec256 => I256,
    inner_constructor = Int256::new_from_i128,
    decimal_places    = 18
}
