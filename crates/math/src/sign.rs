use bnum::types::{U256, U512};

/// Describes a number that can take on negative values.
/// Zero is considered non-negative, for which this should return `false`.
pub trait Sign {
    fn abs(self) -> Self;

    fn is_negative(&self) -> bool;
}

macro_rules! impl_sign {
    ($t:ty) => {
        impl Sign for $t {
            fn abs(self) -> Self {
                self
            }

            fn is_negative(&self) -> bool {
                false
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(
            impl_sign!($t);
        )+
    };
}

impl_sign!(u8, u16, u32, u64, u128, U256, U512);
