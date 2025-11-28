use grug::{Dec, MathResult, Uint128};

pub const ONE_OVER_NATURAL_LOG_OF_TWO: u64 = 1442695040888963407;
pub const LOG2_OF_TEN: u64 = 3321928094887362348;

pub trait UnsignedDecimalConstant {
    const INNER_VALUE: u128;
    const DECIMAL_PLACES: u32;

    fn to_decimal_value<const S: u32>() -> MathResult<Dec<u128, S>> {
        Dec::<u128, S>::checked_from_atomics(Uint128::new(Self::INNER_VALUE), Self::DECIMAL_PLACES)
    }
}

pub struct NaturalLogOfTwo {}
impl UnsignedDecimalConstant for NaturalLogOfTwo {
    const DECIMAL_PLACES: u32 = 24;
    const INNER_VALUE: u128 = 693147180559945309417232;
}

pub struct NaturalLogOfTen {}
impl UnsignedDecimalConstant for NaturalLogOfTen {
    const DECIMAL_PLACES: u32 = 24;
    const INNER_VALUE: u128 = 2302585092994045684017991;
}
