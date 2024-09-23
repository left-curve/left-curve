use {
    crate::NumberConst,
    bnum::types::{U256, U512},
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
