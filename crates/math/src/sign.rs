use {
    crate::NumberConst,
    bnum::types::{I256, I512, U256, U512},
};

/// Describes a number that can take on negative values.
/// Zero is considered non-negative, for which this should return `false`.
pub trait Sign {
    fn abs(self) -> Self;

    fn is_negative(&self) -> bool;
}

// ----------------------------------- unsigned ------------------------------------

macro_rules! impl_sign_unsigned {
    ($($t:ty),+) => {
        $(
            impl Sign for $t {
                fn abs(self) -> Self {
                    self
                }

                fn is_negative(&self) -> bool {
                    false
                }
            }
        )+
    };
}

impl_sign_unsigned!(u8, u16, u32, u64, u128, U256, U512);

// ----------------------------------- signed ------------------------------------

macro_rules! impl_sign_signed {
    ($($t:ty),+) => {
        $(
            impl Sign for $t {
                fn abs(self) -> Self {
                    self.abs()
                }

                fn is_negative(&self) -> bool {
                    *self < Self::ZERO
                }
            }
        )+
    };
}

impl_sign_signed!(i8, i16, i32, i64, i128, I256, I512);
