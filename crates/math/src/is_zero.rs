use {
    crate::NumberConst,
    bnum::types::{U256, U512},
};

/// Describes a number that can be compared to zero.
pub trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($t:ty) => {
        impl IsZero for $t
        where
            $t: NumberConst,
        {
            fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }
        }
    };
    ($($t:ty),+) => {
        $(
            impl_is_zero!($t);
        )+
    };
}

impl_is_zero!(u8, u16, u32, u64, u128, U256, U512);
