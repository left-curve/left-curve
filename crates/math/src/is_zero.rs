use {
    crate::NumberConst,
    bnum::types::{I256, I512, U256, U512},
};

pub trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($($t:ty),+) => {
        $(impl IsZero for $t
        where
            $t: NumberConst ,
        {
            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }
        })+
    };
}

impl_is_zero!(u8, u16, u32, u64, u128, U256, U512);
impl_is_zero!(i8, i16, i32, i64, i128, I256, I512);
